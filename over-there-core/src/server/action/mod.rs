mod handler;

use crate::{
    reply, server::state::ServerState, Content, Header,
    LazilyTransformedRequest, Msg, MsgError, Reply, ReplyError, Request,
    TransformRequestError,
};
use futures::future::{BoxFuture, FutureExt};
use log::trace;
use over_there_derive::Error;
use std::collections::hash_map::Entry;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;
use tokio::{runtime::Handle, sync::mpsc};

#[derive(Debug, Error)]
pub enum ActionError {
    MsgError(MsgError),
    RespondFailed,
    UnexpectedContent,
}

struct OriginSender<T> {
    tx: mpsc::Sender<T>,
    addr: SocketAddr,
}

impl OriginSender<Vec<u8>> {
    pub fn new(tx: mpsc::Sender<Vec<u8>>, addr: SocketAddr) -> Self {
        Self { tx, addr }
    }

    pub async fn send(
        &mut self,
        data: Vec<u8>,
    ) -> Result<(), mpsc::error::SendError<Vec<u8>>> {
        self.tx.send(data).await
    }
}

impl OriginSender<(Vec<u8>, SocketAddr)> {
    pub fn new(
        tx: mpsc::Sender<(Vec<u8>, SocketAddr)>,
        addr: SocketAddr,
    ) -> Self {
        Self { tx, addr }
    }

    pub async fn send(
        &mut self,
        data: Vec<u8>,
    ) -> Result<(), mpsc::error::SendError<(Vec<u8>, SocketAddr)>> {
        self.tx.send((data, self.addr)).await
    }
}

pub struct Executor<T> {
    origin_sender: OriginSender<T>,
    max_depth: u8,
}

impl<T> Executor<T> {
    /// Represents the maximum depth to process nested sequential and batch operations
    pub const DEFAULT_MAX_DEPTH: u8 = 5;
}

impl Executor<Vec<u8>> {
    pub fn new(
        tx: mpsc::Sender<Vec<u8>>,
        origin_addr: SocketAddr,
        max_depth: u8,
    ) -> Self {
        let origin_sender = OriginSender::<Vec<u8>>::new(tx, origin_addr);
        Self {
            origin_sender,
            max_depth,
        }
    }

    pub async fn execute(
        self,
        state: Arc<ServerState>,
        msg: Msg,
    ) -> Result<(), ActionError> {
        let header = msg.header.clone();
        let origin_sender = self.origin_sender;
        let addr = origin_sender.addr;

        let reply = validate_route_and_execute(
            state,
            msg.content,
            addr,
            self.max_depth,
        )
        .await?;

        match reply {
            Reply::Ignore => Ok(()),
            _ => Self::respond(reply, header, origin_sender).await,
        }
    }

    async fn respond(
        reply: Reply,
        parent_header: Header,
        mut origin_sender: OriginSender<Vec<u8>>,
    ) -> Result<(), ActionError> {
        let new_msg = Msg::new(Content::Reply(reply), Some(parent_header));
        let data = new_msg.to_vec().map_err(ActionError::MsgError)?;

        origin_sender
            .send(data)
            .await
            .map_err(|_| ActionError::RespondFailed)
    }
}

impl Executor<(Vec<u8>, SocketAddr)> {
    pub fn new(
        tx: mpsc::Sender<(Vec<u8>, SocketAddr)>,
        origin_addr: SocketAddr,
        max_depth: u8,
    ) -> Self {
        let origin_sender =
            OriginSender::<(Vec<u8>, SocketAddr)>::new(tx, origin_addr);
        Self {
            origin_sender,
            max_depth,
        }
    }

    pub async fn execute(
        self,
        state: Arc<ServerState>,
        msg: Msg,
    ) -> Result<(), ActionError> {
        let header = msg.header.clone();
        let origin_sender = self.origin_sender;
        let addr = origin_sender.addr;

        let reply = validate_route_and_execute(
            state,
            msg.content,
            addr,
            self.max_depth,
        )
        .await?;

        match reply {
            Reply::Ignore => Ok(()),
            _ => Self::respond(reply, header, origin_sender).await,
        }
    }

    async fn respond(
        reply: Reply,
        parent_header: Header,
        mut origin_sender: OriginSender<(Vec<u8>, SocketAddr)>,
    ) -> Result<(), ActionError> {
        let new_msg = Msg::new(Content::Reply(reply), Some(parent_header));
        let data = new_msg.to_vec().map_err(ActionError::MsgError)?;

        origin_sender
            .send(data)
            .await
            .map_err(|_| ActionError::RespondFailed)
    }
}

async fn validate_route_and_execute(
    state: Arc<ServerState>,
    content: Content,
    origin: SocketAddr,
    max_depth: u8,
) -> Result<Reply, ActionError> {
    trace!("Executing content: {:?}", content);

    let request = content.to_request().ok_or(ActionError::UnexpectedContent)?;
    update_origin_last_touched(Arc::clone(&state), origin).await;
    Ok(route_and_execute(state, request, max_depth).await)
}

/// Determines the appropriate handler for a request and executes it
///
/// Returns a boxed future as requests like Sequence and Batch will
/// recursively call this function
fn route_and_execute(
    state: Arc<ServerState>,
    request: Request,
    max_depth: u8,
) -> BoxFuture<'static, Reply> {
    async move {
        if max_depth == 0 {
            Reply::Error(ReplyError::from("Reached maximum nested depth"))
        } else {
            match request {
                Request::Heartbeat => {
                    handler::heartbeat::heartbeat().await;
                    Reply::Heartbeat
                }
                Request::Version => {
                    Reply::Version(handler::version::version().await)
                }
                Request::Capabilities => Reply::Capabilities(
                    handler::capabilities::capabilities().await,
                ),
                Request::OpenFile(args) => handler::fs::open_file(state, &args)
                    .await
                    .map(Reply::FileOpened)
                    .unwrap_or_else(Reply::from),
                Request::CloseFile(args) => {
                    handler::fs::close_file(state, &args)
                        .await
                        .map(Reply::FileClosed)
                        .unwrap_or_else(Reply::from)
                }
                Request::RenameUnopenedFile(args) => {
                    handler::fs::rename_unopened_file(state, &args)
                        .await
                        .map(Reply::UnopenedFileRenamed)
                        .unwrap_or_else(Reply::from)
                }
                Request::RenameFile(args) => {
                    handler::fs::rename_file(state, &args)
                        .await
                        .map(Reply::FileRenamed)
                        .unwrap_or_else(Reply::from)
                }
                Request::RemoveUnopenedFile(args) => {
                    handler::fs::remove_unopened_file(state, &args)
                        .await
                        .map(Reply::UnopenedFileRemoved)
                        .unwrap_or_else(Reply::from)
                }
                Request::RemoveFile(args) => {
                    handler::fs::remove_file(state, &args)
                        .await
                        .map(Reply::FileRemoved)
                        .unwrap_or_else(Reply::from)
                }
                Request::ReadFile(args) => handler::fs::read_file(state, &args)
                    .await
                    .map(Reply::FileContents)
                    .unwrap_or_else(Reply::from),
                Request::WriteFile(args) => {
                    handler::fs::write_file(state, &args)
                        .await
                        .map(Reply::FileWritten)
                        .unwrap_or_else(Reply::from)
                }
                Request::CreateDir(args) => {
                    handler::fs::create_dir(state, &args)
                        .await
                        .map(Reply::DirCreated)
                        .unwrap_or_else(Reply::from)
                }
                Request::RenameDir(args) => {
                    handler::fs::rename_dir(state, &args)
                        .await
                        .map(Reply::DirRenamed)
                        .unwrap_or_else(Reply::from)
                }
                Request::RemoveDir(args) => {
                    handler::fs::remove_dir(state, &args)
                        .await
                        .map(Reply::DirRemoved)
                        .unwrap_or_else(Reply::from)
                }
                Request::ListDirContents(args) => {
                    handler::fs::list_dir_contents(state, &args)
                        .await
                        .map(Reply::DirContentsList)
                        .unwrap_or_else(Reply::from)
                }
                Request::ExecProc(args) => {
                    handler::proc::exec_proc(state, &args)
                        .await
                        .map(Reply::ProcStarted)
                        .unwrap_or_else(Reply::from)
                }
                Request::WriteProcStdin(args) => {
                    handler::proc::write_proc_stdin(state, &args)
                        .await
                        .map(Reply::ProcStdinWritten)
                        .unwrap_or_else(Reply::from)
                }
                Request::ReadProcStdout(args) => {
                    handler::proc::read_proc_stdout(state, &args)
                        .await
                        .map(Reply::ProcStdoutContents)
                        .unwrap_or_else(Reply::from)
                }
                Request::ReadProcStderr(args) => {
                    handler::proc::read_proc_stderr(state, &args)
                        .await
                        .map(Reply::ProcStderrContents)
                        .unwrap_or_else(Reply::from)
                }
                Request::ReadProcStatus(args) => {
                    handler::proc::read_proc_status(state, &args)
                        .await
                        .map(Reply::ProcStatus)
                        .unwrap_or_else(Reply::from)
                }
                Request::KillProc(args) => {
                    handler::proc::kill_proc(state, &args)
                        .await
                        .map(Reply::ProcKilled)
                        .unwrap_or_else(Reply::from)
                }
                Request::InternalDebug(args) => Reply::InternalDebug(
                    handler::internal_debug::internal_debug(state, &args).await,
                ),
                Request::Sequence(mut args) => {
                    let mut results: Vec<Reply> = vec![];
                    for op in args.operations.drain(..) {
                        results.push(
                            match try_transform_request(op, results.last()) {
                                Ok(req) => {
                                    route_and_execute(
                                        Arc::clone(&state),
                                        req,
                                        max_depth - 1,
                                    )
                                    .await
                                }
                                Err(x) => Reply::Error(ReplyError::from(
                                    format!("{}", x),
                                )),
                            },
                        );
                    }

                    Reply::Sequence(reply::SequenceArgs { results })
                }
                Request::Batch(mut args) => {
                    use futures::future::join_all;

                    let results: Vec<Reply> =
                        join_all(args.operations.drain(..).map(|req| {
                            Handle::current().spawn(route_and_execute(
                                Arc::clone(&state),
                                req,
                                max_depth - 1,
                            ))
                        }))
                        .await
                        .drain(..)
                        .map(|r| {
                            r.unwrap_or_else(|x| {
                                Reply::Error(From::from(format!("{}", x)))
                            })
                        })
                        .collect();
                    Reply::Batch(reply::BatchArgs { results })
                }

                // TODO: Move to handler function that can be tested
                //       and have logging
                Request::Custom(args) => match &state.custom_handler.as_ref() {
                    Some(ch) => ch
                        .invoke(args)
                        .await
                        .map(Reply::Custom)
                        .unwrap_or_else(Reply::from),
                    None => Reply::Ignore,
                },

                // TODO: Implement forwarding support
                Request::Forward(_) => Reply::Ignore,
            }
        }
    }
    .boxed()
}

#[derive(Debug, Error)]
enum SequenceError {
    Abort,
    Transform(TransformRequestError),
}

fn try_transform_request(
    op: LazilyTransformedRequest,
    previous_reply: Option<&Reply>,
) -> Result<Request, SequenceError> {
    match previous_reply {
        None => Ok(op.into_raw_request()),
        Some(Reply::Error(_)) => Err(SequenceError::Abort),
        Some(reply) => op
            .transform_with_reply(reply)
            .map_err(SequenceError::Transform),
    }
}

/// Update last time we received a message from the connection
async fn update_origin_last_touched(
    state: Arc<ServerState>,
    origin: SocketAddr,
) -> Option<Instant> {
    match state.conns.lock().await.entry(origin) {
        Entry::Occupied(mut e) => Some(e.insert(Instant::now())),
        Entry::Vacant(e) => {
            e.insert(Instant::now());
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::request;

    #[tokio::test]
    async fn route_and_execute_with_sequence_should_execute_request_in_order() {
        let delay = 30;
        let mut state = ServerState::default();
        make_custom_handler_return_time(&mut state, delay);

        let reply = route_and_execute(
            Arc::new(state),
            Request::Sequence(From::from(vec![
                Request::Custom(From::from(Vec::<u8>::new()))
                    .into_lazily_transformed(vec![]),
                Request::Custom(From::from(Vec::<u8>::new()))
                    .into_lazily_transformed(vec![]),
                Request::Custom(From::from(Vec::<u8>::new()))
                    .into_lazily_transformed(vec![]),
            ])),
            2,
        )
        .await;

        match reply {
            Reply::Sequence(args) => {
                let times = parse_replies_as_times(args.results);

                // Time between each in sequence should be ~30 millis
                assert!(
                    times[0] < times[1] && times[1] - times[0] <= 40,
                    "{} - {} not ~ 30 millis apart",
                    times[1],
                    times[0],
                );
                assert!(
                    times[1] < times[2] && times[2] - times[1] <= 40,
                    "{} - {} not ~ 30 millis apart",
                    times[2],
                    times[1]
                );
            }
            x => panic!("Unexpected reply: {:?}", x),
        }
    }

    #[tokio::test]
    async fn route_and_execute_with_sequence_should_abort_later_operations_if_one_fails(
    ) {
        let mut state = ServerState::default();

        // Set custom handler to fail if it receives any data, but succeed
        // if receives empty data
        state.set_custom_handler(From::from(
            move |req: request::CustomArgs| async move {
                if req.data.is_empty() {
                    Ok(reply::CustomArgs { data: vec![] })
                } else {
                    Err("Bad data".into())
                }
            },
        ));

        let reply = route_and_execute(
            Arc::new(state),
            Request::Sequence(From::from(vec![
                Request::Custom(From::from(Vec::<u8>::new()))
                    .into_lazily_transformed(vec![]),
                Request::Custom(From::from(vec![1, 2, 3]))
                    .into_lazily_transformed(vec![]),
                Request::Custom(From::from(Vec::<u8>::new()))
                    .into_lazily_transformed(vec![]),
            ])),
            2,
        )
        .await;

        match reply {
            Reply::Sequence(args) => {
                match &args.results[0] {
                    Reply::Custom(_) => (),
                    x => panic!("Unexpected reply in sequence[0]: {:?}", x),
                }
                match &args.results[1] {
                    Reply::Error(_) => (),
                    x => panic!("Unexpected reply in batch[1]: {:?}", x),
                }
                match &args.results[2] {
                    Reply::Error(_) => (),
                    x => panic!("Unexpected reply in batch[2]: {:?}", x),
                }
            }
            x => panic!("Unexpected reply: {:?}", x),
        }
    }

    // TODO: Batch operations may run concurrently, but the delay_for tactic
    //       appears to not let other tasks start, even when using
    //       Handle.spawn(...); so, we aren't able to validate that batching
    //       can occur with this setup
    #[tokio::test]
    #[ignore]
    async fn route_and_execute_with_batch_should_request_in_parallel() {
        let delay = 30;
        let mut state = ServerState::default();
        make_custom_handler_return_time(&mut state, delay);

        let reply = route_and_execute(
            Arc::new(state),
            Request::Batch(From::from(vec![
                Request::Custom(From::from(Vec::<u8>::new())),
                Request::Custom(From::from(Vec::<u8>::new())),
                Request::Custom(From::from(Vec::<u8>::new())),
            ])),
            2,
        )
        .await;

        match reply {
            Reply::Batch(args) => {
                let times = parse_replies_as_times(args.results);

                // The time difference between all parallel requests being
                // completed should be minimal
                assert!(
                    time_diff(times[0], times[1]) < 10,
                    "Batch[0,1]: Time between parallel requests was too large: {}",
                    time_diff(times[0], times[1])
                );
                assert!(
                    time_diff(times[0], times[2]) < 10,
                    "Batch[0,2]: Time between parallel requests was too large: {}",
                    time_diff(times[0], times[2])
                );
                assert!(
                    time_diff(times[1], times[2]) < 10,
                    "Batch[1,2]: Time between parallel requests was too large: {}",
                    time_diff(times[1], times[2])
                );
            }
            x => panic!("Unexpected reply: {:?}", x),
        }
    }

    #[tokio::test]
    async fn route_and_execute_with_batch_should_continue_running_requests_if_one_fails(
    ) {
        let mut state = ServerState::default();

        // Set custom handler to fail if it receives any data, but succeed
        // if receives empty data
        state.set_custom_handler(From::from(
            move |req: request::CustomArgs| async move {
                if req.data.is_empty() {
                    Ok(reply::CustomArgs { data: vec![] })
                } else {
                    Err("Bad data".into())
                }
            },
        ));

        let reply = route_and_execute(
            Arc::new(state),
            Request::Batch(From::from(vec![
                Request::Custom(From::from(Vec::<u8>::new())),
                Request::Custom(From::from(vec![1, 2, 3])),
                Request::Custom(From::from(Vec::<u8>::new())),
            ])),
            2,
        )
        .await;

        match reply {
            Reply::Batch(args) => {
                match &args.results[0] {
                    Reply::Custom(_) => (),
                    x => panic!("Unexpected reply in batch[0]: {:?}", x),
                }
                match &args.results[1] {
                    Reply::Error(_) => (),
                    x => panic!("Unexpected reply in batch[1]: {:?}", x),
                }
                match &args.results[2] {
                    Reply::Custom(_) => (),
                    x => panic!("Unexpected reply in batch[2]: {:?}", x),
                }
            }
            x => panic!("Unexpected reply: {:?}", x),
        }
    }

    #[tokio::test]
    async fn update_origin_last_touched_should_create_a_new_entry_if_missing() {
        let state = Arc::new(ServerState::default());
        let origin: SocketAddr = "127.0.0.1:60123".parse().unwrap();

        let now = Instant::now();
        let state_2 = Arc::clone(&state);
        update_origin_last_touched(state_2, origin).await;

        let new_touched = *state
            .conns
            .lock()
            .await
            .get(&origin)
            .expect("No entry was made");
        assert!(new_touched >= now, "Inserted time was in the past");
    }

    #[tokio::test]
    async fn update_origin_last_touched_should_update_existing_entry() {
        let state = Arc::new(ServerState::default());
        let origin: SocketAddr = "127.0.0.1:60123".parse().unwrap();

        let now = Instant::now();
        state.conns.lock().await.insert(origin, now);

        let state_2 = Arc::clone(&state);
        let old_touched = update_origin_last_touched(state_2, origin).await;

        let new_touched = *state
            .conns
            .lock()
            .await
            .get(&origin)
            .expect("No entry was made");
        assert!(
            new_touched >= old_touched.expect("Old entry was not returned"),
            "Inserted time was in the past"
        );
    }

    fn time_diff(time1: u128, time2: u128) -> u128 {
        time1
            .checked_sub(time2)
            .or(time2.checked_sub(time1))
            .expect("Bad time diff")
    }

    fn make_custom_handler_return_time(state: &mut ServerState, delay: u64) {
        state.set_custom_handler(From::from(move |_| {
            use std::time::{Duration, SystemTime, UNIX_EPOCH};
            tokio::time::delay_for(Duration::from_millis(delay)).map(|_| {
                Ok(reply::CustomArgs {
                    data: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_millis()
                        .to_be_bytes()
                        .to_vec(),
                })
            })
        }));
    }

    fn parse_replies_as_times(replies: Vec<Reply>) -> Vec<u128> {
        replies
            .iter()
            .map(|r| match r {
                Reply::Custom(args) => {
                    use std::convert::TryInto;
                    let (int_bytes, _) =
                        args.data.split_at(std::mem::size_of::<u128>());
                    u128::from_be_bytes(int_bytes.try_into().unwrap())
                }
                x => panic!("Unexpected reply in sequence: {:?}", x),
            })
            .collect()
    }
}
