mod handler;

use crate::{
    msg::{content::Content, Header, Msg, MsgError},
    server::state::ServerState,
};
use log::trace;
use over_there_derive::Error;
use over_there_wire::OutboundWireError;
use std::future::Future;

#[derive(Debug, Error)]
pub enum ActionError {
    MsgError(MsgError),
    OutboundWireError(OutboundWireError),
    Unknown,
}

/// Evaluate a message's content and potentially respond using the provided responder
pub async fn execute<F, R>(state: &mut ServerState, msg: &Msg, send: F) -> Result<(), ActionError>
where
    F: FnMut(&[u8]) -> R,
    R: Future<Output = Result<(), OutboundWireError>>,
{
    trace!("Received msg: {:?}", msg);

    let header = msg.header.clone();
    let do_respond = move |content: Content| respond(content, header, send);

    match &msg.content {
        Content::Heartbeat => handler::heartbeat::heartbeat(do_respond).await,
        Content::DoGetVersion => handler::version::do_get_version(do_respond).await,
        Content::DoGetCapabilities => handler::capabilities::do_get_capabilities(do_respond).await,
        Content::DoOpenFile(args) => handler::file::do_open_file(state, args, do_respond).await,
        Content::DoReadFile(args) => handler::file::do_read_file(state, args, do_respond).await,
        Content::DoWriteFile(args) => handler::file::do_write_file(state, args, do_respond).await,
        Content::DoListDirContents(args) => {
            handler::file::do_list_dir_contents(state, args, do_respond).await
        }
        Content::DoExecProc(args) => handler::proc::do_exec_proc(state, args, do_respond).await,
        Content::DoWriteStdin(args) => handler::proc::do_write_stdin(state, args, do_respond).await,
        Content::DoGetStdout(args) => handler::proc::do_get_stdout(state, args, do_respond).await,
        Content::DoGetStderr(args) => handler::proc::do_get_stderr(state, args, do_respond).await,
        Content::DoKillProc(args) => handler::proc::do_kill_proc(state, args, do_respond).await,
        _ => Err(ActionError::Unknown),
    }
}

/// Sends a response to the originator of a msg
async fn respond<F, R>(content: Content, parent_header: Header, f: F) -> Result<(), ActionError>
where
    F: FnMut(&[u8]) -> R,
    R: Future<Output = Result<(), OutboundWireError>>,
{
    let new_msg = Msg::from((content, parent_header));
    let data = new_msg.to_vec().map_err(ActionError::MsgError)?;

    f(&data).await.map_err(ActionError::OutboundWireError)
}
