use crate::{action::ActionError, msg::Msg, state::State};
use over_there_transport::Responder;

pub fn unknown<R: Responder, S: State>(
    _state: &mut S,
    _msg: &Msg,
    _responder: &R,
) -> Result<(), ActionError> {
    Err(ActionError::Unknown)
}
