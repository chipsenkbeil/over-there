use crate::{
    action::{ActionError, ActionState},
    msg::{content::Content, Msg},
};
use over_there_transport::NetSend;
use std::time::Instant;

/// Requests a new heartbeat to confirm remote endpoint is alive
pub fn heartbeat_request<NS: NetSend>(
    _state: &mut ActionState,
    msg: Msg,
    ns: NS,
) -> Result<(), ActionError<NS::TSendData>> {
    let new_msg = Msg::from((Content::HeartbeatResponse, msg));
    let data = new_msg.to_vec().map_err(ActionError::MsgError)?;
    ns.send(&data).map_err(ActionError::SendError)
}

/// Updates the last heartbeat we have received
pub fn heartbeat_response<NS: NetSend>(
    state: &mut ActionState,
    _msg: Msg,
    _ns: NS,
) -> Result<(), ActionError<NS::TSendData>> {
    state.last_heartbeat = Instant::now();
    Ok(())
}
