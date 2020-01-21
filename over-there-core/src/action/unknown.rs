use crate::{action::ActionError, msg::Msg};
use over_there_transport::NetSend;

pub fn unknown<NS: NetSend>(_msg: Msg, _ns: NS) -> Result<(), ActionError<NS::TSendData>> {
    Ok(())
}
