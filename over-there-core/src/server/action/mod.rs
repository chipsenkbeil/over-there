mod handler;

use crate::{
    msg::{
        content::{
            Content, ErrorArgs, LazilyTransformedContent, SequenceResultsArgs,
        },
        Header, Msg, MsgError,
    },
    server::state::ServerState,
};
use log::trace;
use over_there_derive::Error;
use std::collections::hash_map::Entry;
use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{mpsc, Mutex};

#[derive(Debug, Error)]
pub enum ActionError {
    MsgError(MsgError),
    NestedSequenceFound,
    RespondFailed,
    Unknown,
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
}

impl Executor<Vec<u8>> {
    pub fn new(tx: mpsc::Sender<Vec<u8>>, origin_addr: SocketAddr) -> Self {
        let origin_sender = OriginSender::<Vec<u8>>::new(tx, origin_addr);
        Self { origin_sender }
    }

    pub async fn execute(
        self,
        state: Arc<ServerState>,
        msg: Msg,
    ) -> Result<(), ActionError> {
        let header = msg.header.clone();
        let origin_sender = self.origin_sender;
        let addr = origin_sender.addr;

        Self::execute_impl(state, msg.content, addr, move |content: Content| {
            trace!("Response: {:?}", content);
            Self::respond(content, header, origin_sender)
        })
        .await
    }

    async fn respond(
        content: Content,
        parent_header: Header,
        mut origin_sender: OriginSender<Vec<u8>>,
    ) -> Result<(), ActionError> {
        let new_msg = Msg::from((content, parent_header));
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
    ) -> Self {
        let origin_sender =
            OriginSender::<(Vec<u8>, SocketAddr)>::new(tx, origin_addr);
        Self { origin_sender }
    }

    pub async fn execute(
        self,
        state: Arc<ServerState>,
        msg: Msg,
    ) -> Result<(), ActionError> {
        let header = msg.header.clone();
        let origin_sender = self.origin_sender;
        let addr = origin_sender.addr;

        Self::execute_impl(state, msg.content, addr, move |content: Content| {
            trace!("Response: {:?}", content);
            Self::respond(content, header, origin_sender)
        })
        .await
    }

    async fn respond(
        content: Content,
        parent_header: Header,
        mut origin_sender: OriginSender<(Vec<u8>, SocketAddr)>,
    ) -> Result<(), ActionError> {
        let new_msg = Msg::from((content, parent_header));
        let data = new_msg.to_vec().map_err(ActionError::MsgError)?;

        origin_sender
            .send(data)
            .await
            .map_err(|_| ActionError::RespondFailed)
    }
}

impl<T> Executor<T> {
    /// Evaluate a message's content and potentially respond using the provided responder
    async fn execute_impl<F, R>(
        state: Arc<ServerState>,
        mut content: Content,
        origin: SocketAddr,
        do_respond: F,
    ) -> Result<(), ActionError>
    where
        F: FnOnce(Content) -> R,
        R: Future<Output = Result<(), ActionError>>,
    {
        trace!("Executing content: {:?}", content);
        update_origin_last_touched(Arc::clone(&state), origin).await;

        match &mut content {
            Content::DoSequence(args) => {
                Self::execute_sequence(
                    state,
                    args.operations.drain(..).collect(),
                    do_respond,
                )
                .await
            }
            _ => Self::execute_impl_normal(state, content, do_respond).await,
        }
    }

    /// Evaluate content that does not contain other content (e.g. sequence)
    async fn execute_impl_normal<F, R>(
        state: Arc<ServerState>,
        content: Content,
        do_respond: F,
    ) -> Result<(), ActionError>
    where
        F: FnOnce(Content) -> R,
        R: Future<Output = Result<(), ActionError>>,
    {
        match &content {
            Content::Heartbeat => {
                handler::heartbeat::heartbeat(do_respond).await
            }
            Content::DoGetVersion => {
                handler::version::do_get_version(do_respond).await
            }
            Content::DoGetCapabilities => {
                handler::capabilities::do_get_capabilities(do_respond).await
            }
            Content::DoOpenFile(args) => {
                handler::fs::do_open_file(state, args, do_respond).await
            }
            Content::DoCloseFile(args) => {
                handler::fs::do_close_file(state, args, do_respond).await
            }
            Content::DoRenameUnopenedFile(args) => {
                handler::fs::do_rename_unopened_file(state, args, do_respond)
                    .await
            }
            Content::DoRenameFile(args) => {
                handler::fs::do_rename_file(state, args, do_respond).await
            }
            Content::DoRemoveUnopenedFile(args) => {
                handler::fs::do_remove_unopened_file(state, args, do_respond)
                    .await
            }
            Content::DoRemoveFile(args) => {
                handler::fs::do_remove_file(state, args, do_respond).await
            }
            Content::DoReadFile(args) => {
                handler::fs::do_read_file(state, args, do_respond).await
            }
            Content::DoWriteFile(args) => {
                handler::fs::do_write_file(state, args, do_respond).await
            }
            Content::DoCreateDir(args) => {
                handler::fs::do_create_dir(state, args, do_respond).await
            }
            Content::DoRenameDir(args) => {
                handler::fs::do_rename_dir(state, args, do_respond).await
            }
            Content::DoRemoveDir(args) => {
                handler::fs::do_remove_dir(state, args, do_respond).await
            }
            Content::DoListDirContents(args) => {
                handler::fs::do_list_dir_contents(state, args, do_respond).await
            }
            Content::DoExecProc(args) => {
                handler::proc::do_exec_proc(state, args, do_respond).await
            }
            Content::DoWriteStdin(args) => {
                handler::proc::do_write_stdin(state, args, do_respond).await
            }
            Content::DoGetStdout(args) => {
                handler::proc::do_get_stdout(state, args, do_respond).await
            }
            Content::DoGetStderr(args) => {
                handler::proc::do_get_stderr(state, args, do_respond).await
            }
            Content::DoGetProcStatus(args) => {
                handler::proc::do_get_proc_status(state, args, do_respond).await
            }
            Content::DoKillProc(args) => {
                handler::proc::do_kill_proc(state, args, do_respond).await
            }
            Content::InternalDebug(args) => {
                handler::internal_debug::internal_debug(state, args, do_respond)
                    .await
            }
            Content::DoSequence(_) => Err(ActionError::NestedSequenceFound),
            _ => Err(ActionError::Unknown),
        }
    }

    /// Evaluate a sequence of requests by executing the first, piping its
    /// output response to the second, and repeat until completed or an error
    /// occurs
    ///
    /// If
    async fn execute_sequence<F, R>(
        state: Arc<ServerState>,
        operations: Vec<LazilyTransformedContent>,
        do_respond: F,
    ) -> Result<(), ActionError>
    where
        F: FnOnce(Content) -> R,
        R: Future<Output = Result<(), ActionError>>,
    {
        let (mut tx, mut rx) = tokio::sync::mpsc::channel(operations.len());
        let mut results: Vec<Content> = Vec::new();

        for op in operations.iter() {
            // Transform our operation using the previous content
            // if available, or do no transformation at all
            let op_content = match rx.try_recv() {
                Ok(c) => {
                    // Perform the transformation
                    let x = op.transform_with_base(&c);

                    // Now that we're done, store the previous outgoing
                    // content as one of our results
                    results.push(c);

                    x
                }
                _ => Ok(op.clone().into_raw_content()),
            };

            // If the transformation failed, send a generic error
            // message back to the client and conclude the action
            if let Err(x) = op_content {
                return do_respond(Content::Error(ErrorArgs {
                    msg: format!("Sequencing failed: {}", x),
                }))
                .await;
            }

            // Execute the operation -- note that we don't allow nested sequencing
            let mut tx_2 = tx.clone();
            let do_respond = move |content: Content| {
                use futures::TryFutureExt;
                async move {
                    tx_2.send(content)
                        .map_err(|_| ActionError::RespondFailed)
                        .await
                }
            };
            let result = Self::execute_impl_normal(
                Arc::clone(&state),
                op_content.unwrap(),
                do_respond,
            )
            .await;

            if result.is_err() {
                return result;
            }
        }

        // With all operations complete, we want to send a response back with
        // them included
        do_respond(Content::SequenceResults(SequenceResultsArgs { results }))
            .await
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
