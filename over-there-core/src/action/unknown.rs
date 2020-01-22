use crate::{action::ActionError, msg::Msg, state::State};
use over_there_transport::Responder;

pub fn unknown<R: Responder>(
    _state: &mut State,
    _msg: Msg,
    _responder: &R,
) -> Result<(), ActionError> {
    Err(ActionError::Unknown)
}
