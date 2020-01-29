use crate::{
    msg::Msg,
    server::{action::ActionError, state::ServerState},
};
use log::trace;
use over_there_transport::Responder;

pub fn unknown<R: Responder>(
    _state: &mut ServerState,
    _msg: &Msg,
    _responder: &R,
) -> Result<(), ActionError> {
    trace!("Unknown msg: {:?}", _msg);
    Err(ActionError::Unknown)
}
