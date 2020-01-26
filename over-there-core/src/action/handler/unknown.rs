use crate::{action::ActionError, msg::Msg, state::State};
use log::trace;
use over_there_transport::Responder;

pub fn unknown<R: Responder, S: State>(
    _state: &mut S,
    _msg: &Msg,
    _responder: &R,
) -> Result<(), ActionError> {
    trace!("Unknown msg: {:?}", _msg);
    Err(ActionError::Unknown)
}
