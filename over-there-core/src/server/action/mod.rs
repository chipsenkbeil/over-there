pub mod handler;

use crate::{
    msg::{content::Content, Header, Msg, MsgError},
    server::state::ServerState,
};
use log::trace;
use over_there_derive::Error;
use over_there_transport::{Responder, ResponderError};

#[derive(Debug, Error)]
pub enum ActionError {
    MsgError(MsgError),
    ResponderError(ResponderError),
    Unknown,
}

/// Evaluate a message's content and potentially respond using the provided responder
pub fn execute<R: Responder>(
    state: &mut ServerState,
    msg: &Msg,
    responder: &R,
    mut handler: impl FnMut(&mut ServerState, &Msg, &R) -> Result<(), ActionError>,
) -> Result<(), ActionError> {
    trace!("Received msg: {:?}", msg);
    (handler)(state, msg, responder)
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
mod test_utils {
    use super::*;
    use std::cell::RefCell;

    #[derive(Clone, Debug)]
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
