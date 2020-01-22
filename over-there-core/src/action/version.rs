use crate::{
    action::{self, ActionError},
    msg::{content::Content, Msg},
    state::State,
};
use over_there_transport::NetSend;

/// Handles a request for current version
pub fn version_request<NS: NetSend>(
    _state: &mut State,
    msg: Msg,
    ns: &NS,
) -> Result<(), ActionError> {
    action::respond(
        ns,
        Content::VersionResponse {
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
        msg,
    )
}

/// Updates the last version we have received
pub fn version_response<NS: NetSend>(
    state: &mut State,
    msg: Msg,
    _ns: &NS,
) -> Result<(), ActionError> {
    let version = match msg.content {
        Content::VersionResponse { version } => version,
        _ => return Err(ActionError::UnexpectedContent),
    };

    state.remote_version = version;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::tests::MockNetSend;

    #[test]
    fn version_request_should_send_version_response() {
        let mut state = State::default();
        let msg = Msg::from(Content::VersionRequest);
        let mut ns = MockNetSend::default();

        let result = version_request(&mut state, msg.clone(), &ns);
        assert!(result.is_ok(), "Bad result: {:?}", result);

        let outgoing_msg = Msg::from_slice(&ns.take_last_sent().unwrap()).unwrap();
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
        let mut state = State::default();
        let version = env!("CARGO_PKG_VERSION").to_string();
        let msg = Msg::from(Content::VersionResponse {
            version: version.clone(),
        });
        let mut ns = MockNetSend::default();

        let result = version_response(&mut state, msg.clone(), &ns);
        assert!(result.is_ok(), "Bad result: {:?}", result);

        let last_sent = ns.take_last_sent();
        assert!(last_sent.is_none(), "Unexpected last sent {:?}", last_sent);
        assert!(state.remote_version == version);
    }
}
