use crate::{
    msg::{content::Content, Msg},
    server::{
        action::{self, ActionError},
        state::ServerState,
    },
};
use log::debug;
use over_there_transport::Responder;

pub fn version_request<R: Responder>(
    _state: &mut ServerState,
    msg: &Msg,
    responder: &R,
) -> Result<(), ActionError> {
    debug!(
        "Got version request! Sending response using {:?}",
        responder
    );
    action::respond(
        responder,
        Content::VersionResponse {
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
        msg.header.clone(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::action::test_utils::MockResponder;

    #[test]
    fn version_request_should_send_version_response() {
        let mut state = ServerState::default();
        let msg = Msg::from(Content::VersionRequest);
        let mut responder = MockResponder::default();

        let result = version_request(&mut state, &msg, &responder);
        assert!(result.is_ok(), "Bad result: {:?}", result);

        let outgoing_msg = Msg::from_slice(&responder.take_last_sent().unwrap()).unwrap();
        assert_eq!(outgoing_msg.parent_header, Some(msg.header));
        assert_eq!(
            outgoing_msg.content,
            Content::VersionResponse {
                version: env!("CARGO_PKG_VERSION").to_string(),
            }
        );
    }
}
