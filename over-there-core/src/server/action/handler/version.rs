use crate::{
    msg::content::{version::VersionArgs, Content},
    server::action::ActionError,
};
use log::debug;

pub fn do_get_version(
    respond: impl FnOnce(Content) -> Result<(), ActionError>,
) -> Result<(), ActionError> {
    debug!("version_request");
    respond(Content::Version(VersionArgs {
        version: env!("CARGO_PKG_VERSION").to_string(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn do_get_version_should_send_version() {
        unimplemented!();
    }
}
