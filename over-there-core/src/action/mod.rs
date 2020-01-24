mod handler;

use crate::msg::{
    content::{Content, ContentType},
    Header, Msg, MsgError,
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

/// Evaluate a message's content and potentially respond using the provided
/// netsend component
pub fn execute<R: Responder>(
    state: &mut State,
    msg: &Msg,
    responder: &R,
    mut handler: impl FnMut(&mut State, &Msg, &R) -> Result<(), ActionError>,
) -> Result<(), ActionError> {
    let maybe_callback = msg
        .parent_header
        .as_ref()
        .and_then(|h| state.take_callback(h.id));
    let result = (handler)(state, msg, responder);

    if let Some(mut callback) = maybe_callback {
        callback(msg);
    }

    result
}

/// Looks up an appropriate function pointer for the given content type
pub fn route<R: Responder>(
    content_type: ContentType,
) -> fn(&mut State, &Msg, &R) -> Result<(), ActionError> {
    match content_type {
        ContentType::HeartbeatRequest => handler::heartbeat::heartbeat_request,
        ContentType::HeartbeatResponse => handler::heartbeat::heartbeat_response,

        ContentType::VersionRequest => handler::version::version_request,
        ContentType::VersionResponse => handler::version::version_response,

        // TODO: Remove unknown by completing all other content types
        _ => handler::unknown::unknown,
    }
}

/// Sends a response to the originator of a msg
fn respond<R: Responder>(
    responder: &R,
    content: Content,
    parent_header: Header,
) -> Result<(), ActionError> {
    let new_msg = Msg::from((content, parent_header));
    let data = new_msg.to_vec().map_err(ActionError::MsgError)?;
    responder.send(&data).map_err(ActionError::ResponderError)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;
    use test_utils::MockResponder;

    #[test]
    fn execute_should_invoke_callback_if_it_exists() {
        let mut state = State::default();
        let msg = Msg::from((Content::HeartbeatRequest, Header::default()));
        let responder = MockResponder::default();
        let id = msg.parent_header.clone().unwrap().id;

        let success_1 = Rc::new(RefCell::new(false));
        let success_2 = Rc::clone(&success_1);
        state.add_callback(id, move |_msg| {
            *success_2.borrow_mut() = true;
        });

        assert!(execute(&mut state, &msg, &responder, |_, _, _| { Ok(()) }).is_ok());
        assert!(*success_1.borrow(), "Callback was not invoked!");
    }
}

#[cfg(test)]
mod test_utils {
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
