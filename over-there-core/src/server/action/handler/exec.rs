use crate::{
    msg::content::{
        io::{exec::*, IoErrorArgs},
        Content,
    },
    server::{action::ActionError, proc::LocalProc, state::ServerState},
};
use log::debug;

pub fn do_exec(
    state: &mut ServerState,
    args: &DoExecArgs,
    respond: impl FnOnce(Content) -> Result<(), ActionError>,
) -> Result<(), ActionError> {
    debug!("do_exec: {:?}", args);

    unimplemented!();
}

pub fn do_exec_stdin(
    state: &mut ServerState,
    args: &DoExecStdinArgs,
    respond: impl FnOnce(Content) -> Result<(), ActionError>,
) -> Result<(), ActionError> {
    debug!("do_exec_stdin: {:?}", args);

    unimplemented!();
}

pub fn do_get_exec_stdout(
    state: &mut ServerState,
    args: &DoGetExecStdoutArgs,
    respond: impl FnOnce(Content) -> Result<(), ActionError>,
) -> Result<(), ActionError> {
    debug!("do_get_exec_stdout: {:?}", args);

    unimplemented!();
}

pub fn do_get_exec_stderr(
    state: &mut ServerState,
    args: &DoGetExecStderrArgs,
    respond: impl FnOnce(Content) -> Result<(), ActionError>,
) -> Result<(), ActionError> {
    debug!("do_get_exec_stderr: {:?}", args);

    unimplemented!();
}

pub fn do_exec_kill(
    state: &mut ServerState,
    args: &DoExecKillArgs,
    respond: impl FnOnce(Content) -> Result<(), ActionError>,
) -> Result<(), ActionError> {
    debug!("do_exec_kill: {:?}", args);

    unimplemented!();
}

#[cfg(test)]
mod tests {
    use super::*;
}
