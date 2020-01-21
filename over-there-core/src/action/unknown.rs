use crate::{
    action::{ActionError, ActionState},
    msg::Msg,
};
use over_there_transport::NetSend;

pub fn unknown<NS: NetSend>(
    _state: &mut ActionState,
    _msg: Msg,
    _ns: NS,
) -> Result<(), ActionError<NS::TSendData>> {
    Ok(())
}
