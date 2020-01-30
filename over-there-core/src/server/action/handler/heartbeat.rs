use crate::{msg::content::Content, server::action::ActionError};
use log::debug;

pub fn heartbeat(
    respond: impl FnOnce(Content) -> Result<(), ActionError>,
) -> Result<(), ActionError> {
    debug!("heartbeat_request");
    respond(Content::Heartbeat)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heartbeat_should_update_last_active_status() {
        unimplemented!();
    }
}
