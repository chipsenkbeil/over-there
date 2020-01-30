use crate::{msg::content::Content, server::action::ActionError};
use log::debug;

pub fn version_request(
    respond: impl FnOnce(Content) -> Result<(), ActionError>,
) -> Result<(), ActionError> {
    debug!("version_request");
    respond(Content::VersionResponse {
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
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
