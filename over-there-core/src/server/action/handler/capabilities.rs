use crate::{
    msg::content::{capabilities::*, Content},
    server::action::ActionError,
};
use log::debug;

pub fn do_get_capabilities(
    respond: impl FnOnce(Content) -> Result<(), ActionError>,
) -> Result<(), ActionError> {
    debug!("do_get_capabilities");
    respond(Content::Capabilities(CapabilitiesArgs {
        capabilities: vec![],
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn do_get_capabilities_should_send_capabilities() {
        unimplemented!();
    }
}
