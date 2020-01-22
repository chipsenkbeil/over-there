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
    ns: &NS,
) -> Result<(), ActionError> {
    let new_msg = Msg::from((Content::HeartbeatResponse, msg));
    let data = new_msg.to_vec().map_err(ActionError::MsgError)?;
    ns.send(&data).map_err(ActionError::NetSendError)
}

/// Updates the last heartbeat we have received
pub fn heartbeat_response<NS: NetSend>(
    state: &mut ActionState,
    _msg: Msg,
    _ns: &NS,
) -> Result<(), ActionError> {
    state.last_heartbeat = Instant::now();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::tests::MockNetSend;

    #[test]
    fn heartbeat_request_should_send_heartbeat_response() {
        let mut state = ActionState::default();
        let msg = Msg::from(Content::HeartbeatRequest);
        let mut ns = MockNetSend::default();

        assert!(heartbeat_request(&mut state, msg.clone(), &ns).is_ok());

        let outgoing_msg = Msg::from_slice(&ns.take_last_sent().unwrap()).unwrap();
        assert_eq!(outgoing_msg.parent_header, Some(msg.header));
        assert_eq!(outgoing_msg.content, Content::HeartbeatResponse);
    }

    #[test]
    fn heartbeat_response_should_log_latest_heartbeat() {
        let mut state = ActionState::default();
        let old_last_heartbeat = state.last_heartbeat.clone();
        let msg = Msg::from(Content::HeartbeatResponse);
        let mut ns = MockNetSend::default();

        assert!(heartbeat_response(&mut state, msg.clone(), &ns).is_ok());

        let last_sent = ns.take_last_sent();
        assert!(last_sent.is_none(), "Unexpected last sent {:?}", last_sent);
        assert!(state.last_heartbeat > old_last_heartbeat);
    }
}
