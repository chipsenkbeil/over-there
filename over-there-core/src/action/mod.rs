mod heartbeat;
mod unknown;
mod version;

use crate::{
    msg::{
        content::{Content, ContentType},
        Msg, MsgError,
    },
    state::State,
};
use over_there_derive::Error;
use over_there_transport::{Responder, ResponderError};

#[derive(Debug, Error)]
pub enum ActionError {
    MsgError(MsgError),
    ResponderError(ResponderError),
    UnexpectedContent,
    Unknown,
}

/// Looks up an appropriate function pointer for the given content type
pub fn route<R: Responder>(
    content_type: ContentType,
) -> fn(&mut State, Msg, &R) -> Result<(), ActionError> {
    match content_type {
        ContentType::HeartbeatRequest => heartbeat::heartbeat_request,
        ContentType::HeartbeatResponse => heartbeat::heartbeat_response,

        ContentType::VersionRequest => version::version_request,
        ContentType::VersionResponse => version::version_response,

        // TODO: Remove unknown by completing all other content types
        _ => unknown::unknown,
    }
}

/// Evaluate a message's content and potentially respond using the provided
/// netsend component
pub fn execute<R: Responder>(
    state: &mut State,
    msg: Msg,
    responder: &R,
) -> Result<(), ActionError> {
    (route(ContentType::from(&msg.content)))(state, msg, responder)
}

/// Sends a response to the originator of a msg
pub(crate) fn respond<R: Responder>(
    responder: &R,
    content: Content,
    parent_msg: Msg,
) -> Result<(), ActionError> {
    let new_msg = Msg::from((content, parent_msg));
    let data = new_msg.to_vec().map_err(ActionError::MsgError)?;
    responder.send(&data).map_err(ActionError::ResponderError)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[derive(Clone)]
    pub struct MockResponder {
        last_sent: RefCell<Option<Vec<u8>>>,
    }

    impl MockResponder {
        pub fn take_last_sent(&mut self) -> Option<Vec<u8>> {
            self.last_sent.borrow_mut().take()
        }
    }

    impl Default for MockResponder {
        fn default() -> Self {
            Self {
                last_sent: RefCell::new(None),
            }
        }
    }

    impl Responder for MockResponder {
        fn send(&self, data: &[u8]) -> Result<(), ResponderError> {
            *self.last_sent.borrow_mut() = Some(data.to_vec());
            Ok(())
        }
    }
}
