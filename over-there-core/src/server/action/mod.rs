mod handler;

use crate::{
    msg::{content::Content, Header, Msg, MsgError},
    server::state::ServerState,
};
use log::trace;
use over_there_derive::Error;
use std::collections::hash_map::Entry;
use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;

#[derive(Debug, Error)]
pub enum ActionError {
    MsgError(MsgError),
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

        Self::execute_impl(state, msg, addr, move |content: Content| {
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

        Self::execute_impl(state, msg, addr, move |content: Content| {
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
        msg: Msg,
        origin: SocketAddr,
        do_respond: F,
    ) -> Result<(), ActionError>
    where
        F: FnOnce(Content) -> R,
        R: Future<Output = Result<(), ActionError>>,
    {
        trace!("Executing msg: {:?}", msg);
        update_origin_last_touched(Arc::clone(&state), origin).await;

        match &msg.content {
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
            Content::DoRenameFile(args) => {
                handler::fs::do_rename_file(state, args, do_respond).await
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
            _ => Err(ActionError::Unknown),
        }
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
