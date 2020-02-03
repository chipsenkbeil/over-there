mod handler;

use crate::{
    msg::{content::Content, Header, Msg, MsgError},
    server::state::ServerState,
};
use log::trace;
use over_there_derive::Error;
use over_there_transport::{Responder, ResponderError};

#[derive(Debug, Error)]
pub enum ActionError {
    MsgError(MsgError),
    ResponderError(ResponderError),
    Unknown,
}

/// Evaluate a message's content and potentially respond using the provided responder
pub fn execute<R: Responder>(
    state: &mut ServerState,
    msg: &Msg,
    responder: &R,
) -> Result<(), ActionError> {
    trace!("Received msg: {:?}", msg);

    let header = msg.header.clone();
    let do_respond = |content: Content| respond(responder, content, header);

    match &msg.content {
        Content::Heartbeat => handler::heartbeat::heartbeat(do_respond),
        Content::DoGetVersion => handler::version::do_get_version(do_respond),
        Content::DoGetCapabilities => handler::capabilities::do_get_capabilities(do_respond),
        Content::DoOpenFile(args) => handler::file::do_open_file(state, args, do_respond),
        Content::DoReadFile(args) => handler::file::do_read_file(state, args, do_respond),
        Content::DoWriteFile(args) => handler::file::do_write_file(state, args, do_respond),
        Content::DoListDirContents(args) => {
            handler::file::do_list_dir_contents(state, args, do_respond)
        }
        Content::DoExecProc(args) => handler::proc::do_exec_proc(state, args, do_respond),
        Content::DoWriteStdin(args) => handler::proc::do_write_stdin(state, args, do_respond),
        Content::DoGetStdout(args) => handler::proc::do_get_stdout(state, args, do_respond),
        Content::DoGetStderr(args) => handler::proc::do_get_stderr(state, args, do_respond),
        Content::DoKillProc(args) => handler::proc::do_kill_proc(state, args, do_respond),
        _ => Err(ActionError::Unknown),
    }
}

/// Sends a response to the originator of a msg
fn respond<R: Responder>(
    responder: &R,
    content: Content,
    parent_header: Header,
) -> Result<(), ActionError> {
    let new_msg = Msg::from((content, parent_header));
    let data = new_msg.to_vec().map_err(ActionError::MsgError)?;
    responder.send(&data).map_err(ActionError::ResponderError)
}
