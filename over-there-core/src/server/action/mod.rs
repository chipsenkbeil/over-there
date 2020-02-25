mod handler;

use crate::{
    msg::{content::Content, Header, Msg, MsgError},
    server::state::ServerState,
};
use log::trace;
use over_there_derive::Error;
use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;
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

        Self::execute_impl(state, msg, move |content: Content| {
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

        Self::execute_impl(state, msg, move |content: Content| {
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
        do_respond: F,
    ) -> Result<(), ActionError>
    where
        F: FnOnce(Content) -> R,
        R: Future<Output = Result<(), ActionError>>,
    {
        trace!("Executing msg: {:?}", msg);

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
                handler::file::do_open_file(state, args, do_respond).await
            }
            Content::DoReadFile(args) => {
                handler::file::do_read_file(state, args, do_respond).await
            }
            Content::DoWriteFile(args) => {
                handler::file::do_write_file(state, args, do_respond).await
            }
            Content::DoListDirContents(args) => {
                handler::file::do_list_dir_contents(state, args, do_respond)
                    .await
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
