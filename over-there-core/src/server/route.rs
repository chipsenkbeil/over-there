use crate::{
    action::{handler, ActionError},
    msg::{content::ContentType, Msg},
    server::state::ServerState,
};
use over_there_transport::Responder;

pub fn route<R: Responder>(
    content_type: ContentType,
) -> fn(&mut ServerState, &Msg, &R) -> Result<(), ActionError> {
    match content_type {
        ContentType::HeartbeatRequest => handler::heartbeat::heartbeat_request,
        ContentType::VersionRequest => handler::version::version_request,
        _ => handler::unknown::unknown,
    }
}
