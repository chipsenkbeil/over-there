use crate::{
    action::{self, ActionError},
    client::state::ClientState,
    msg::{content::Content, Msg},
    server::state::ServerState,
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

/// Updates the last version we have received
pub fn version_response<R: Responder>(
    state: &mut ClientState,
    msg: &Msg,
    _responder: &R,
) -> Result<(), ActionError> {
    debug!("Received version!");
    let version = match &msg.content {
        Content::VersionResponse { version } => version,
        _ => return Err(ActionError::UnexpectedContent),
    };

    state.remote_version = version.to_string();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::test_utils::MockResponder;

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

    #[test]
    fn version_response_should_log_remote_version() {
        let mut state = ClientState::default();
        let version = env!("CARGO_PKG_VERSION").to_string();
        let msg = Msg::from(Content::VersionResponse {
            version: version.clone(),
        });
        let mut responder = MockResponder::default();

        let result = version_response(&mut state, &msg, &responder);
        assert!(result.is_ok(), "Bad result: {:?}", result);

        let last_sent = responder.take_last_sent();
        assert!(last_sent.is_none(), "Unexpected last sent {:?}", last_sent);
        assert!(state.remote_version == version);
    }
}
