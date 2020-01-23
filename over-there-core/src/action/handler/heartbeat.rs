use crate::{
    action::{self, ActionError},
    msg::{content::Content, Msg},
    state::State,
};
use over_there_transport::Responder;
use std::time::Instant;

/// Requests a new heartbeat to confirm remote endpoint is alive
pub fn heartbeat_request<R: Responder>(
    _state: &mut State,
    msg: &Msg,
    responder: &R,
) -> Result<(), ActionError> {
    action::respond(responder, Content::HeartbeatResponse, msg.header.clone())
}

/// Updates the last heartbeat we have received
pub fn heartbeat_response<R: Responder>(
    state: &mut State,
    _msg: &Msg,
    _responder: &R,
) -> Result<(), ActionError> {
    state.last_heartbeat = Instant::now();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::test_utils::MockResponder;

    #[test]
    fn heartbeat_request_should_send_heartbeat_response() {
        let mut state = State::default();
        let msg = Msg::from(Content::HeartbeatRequest);
        let mut responder = MockResponder::default();

        let result = heartbeat_request(&mut state, &msg, &responder);
        assert!(result.is_ok(), "Bad result: {:?}", result);

        let outgoing_msg = Msg::from_slice(&responder.take_last_sent().unwrap()).unwrap();
        assert_eq!(outgoing_msg.parent_header, Some(msg.header));
        assert_eq!(outgoing_msg.content, Content::HeartbeatResponse);
    }

    #[test]
    fn heartbeat_response_should_log_latest_heartbeat() {
        let mut state = State::default();
        let old_last_heartbeat = state.last_heartbeat.clone();
        let msg = Msg::from(Content::HeartbeatResponse);
        let mut responder = MockResponder::default();

        let result = heartbeat_response(&mut state, &msg, &responder);
        assert!(result.is_ok(), "Bad result: {:?}", result);

        let last_sent = responder.take_last_sent();
        assert!(last_sent.is_none(), "Unexpected last sent {:?}", last_sent);
        assert!(state.last_heartbeat > old_last_heartbeat);
    }
}
