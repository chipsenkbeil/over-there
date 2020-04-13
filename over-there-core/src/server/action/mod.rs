mod handler;

use crate::{
    reply, server::state::ServerState, Content, Header, Msg, MsgError, Reply,
    ReplyError, Request,
};
use futures::future::{BoxFuture, FutureExt};
use log::trace;
use over_there_derive::Error;
use std::collections::hash_map::Entry;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;

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
        Self::respond(reply, header, origin_sender).await
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
        Self::respond(reply, header, origin_sender).await
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
                        let try_req = if results.is_empty() {
                            Ok(op.into_raw_request())
                        } else {
                            op.transform_with_reply(results.last().unwrap())
                        };

                        let result = match try_req {
                            Ok(req) => {
                                route_and_execute(
                                    Arc::clone(&state),
                                    req,
                                    max_depth - 1,
                                )
                                .await
                            }
                            Err(x) => {
                                Reply::Error(ReplyError::from(format!("{}", x)))
                            }
                        };

                        results.push(result);
                    }

                    Reply::Sequence(reply::SequenceArgs { results })
                }
                Request::Batch(mut args) => {
                    use futures::future::join_all;

                    let results: Vec<Reply> =
                        join_all(args.operations.drain(..).map(|req| {
                            route_and_execute(
                                Arc::clone(&state),
                                req,
                                max_depth - 1,
                            )
                        }))
                        .await;
                    Reply::Batch(reply::BatchArgs { results })
                }
                Request::Custom(_) => unimplemented!(),
                Request::Forward(_) => unimplemented!(),
            }
        }
    }
    .boxed()
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
}
