use crate::{
    action::ActionError,
    msg::{content::Content, Msg},
};
use over_there_transport::NetSend;

pub fn heartbeat_request<NS: NetSend>(msg: Msg, ns: NS) -> Result<(), ActionError<NS::TSendData>> {
    let new_msg = Msg::from((Content::HeartbeatResponse, msg));
    let data = new_msg.to_vec().map_err(ActionError::MsgError)?;
    ns.send(&data).map_err(ActionError::SendError)
}

pub fn heartbeat_response<NS: NetSend>(
    _msg: Msg,
    _ns: NS,
) -> Result<(), ActionError<NS::TSendData>> {
    // TODO: Implement action using msg AND sender
    Ok(())
}
