use crate::{
    msg::{content::Content, Msg},
    server::{
        action::{self, ActionError},
        state::ServerState,
    },
};
use log::debug;
use over_there_transport::Responder;

pub fn heartbeat_request<R: Responder>(
    _state: &mut ServerState,
    msg: &Msg,
    responder: &R,
) -> Result<(), ActionError> {
    debug!(
        "Got heartbeat request! Sending response using {:?}",
        responder
    );
    action::respond(responder, Content::HeartbeatResponse, msg.header.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::action::test_utils::MockResponder;

    #[test]
    fn heartbeat_request_should_send_heartbeat_response() {
        let mut state = ServerState::default();
        let msg = Msg::from(Content::HeartbeatRequest);
        let mut responder = MockResponder::default();

        let result = heartbeat_request(&mut state, &msg, &responder);
        assert!(result.is_ok(), "Bad result: {:?}", result);

        let outgoing_msg = Msg::from_slice(&responder.take_last_sent().unwrap()).unwrap();
        assert_eq!(outgoing_msg.parent_header, Some(msg.header));
        assert_eq!(outgoing_msg.content, Content::HeartbeatResponse);
    }
}
