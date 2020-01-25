use crate::{
    action::{handler, ActionError},
    client::state::ClientState,
    msg::{content::ContentType, Msg},
};
use over_there_transport::Responder;

pub fn route<R: Responder>(
    content_type: ContentType,
) -> fn(&mut ClientState, &Msg, &R) -> Result<(), ActionError> {
    match content_type {
        ContentType::HeartbeatResponse => handler::heartbeat::heartbeat_response,
        ContentType::VersionResponse => handler::version::version_response,
        _ => handler::unknown::unknown,
    }
}
