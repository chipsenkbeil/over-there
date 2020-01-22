use crate::{action::ActionError, msg::Msg, state::State};
use over_there_transport::NetSend;

pub fn unknown<NS: NetSend>(_state: &mut State, _msg: Msg, _ns: &NS) -> Result<(), ActionError> {
    Ok(())
}
