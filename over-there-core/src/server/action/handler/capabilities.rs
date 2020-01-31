use crate::{
    msg::content::{
        capabilities::{CapabilitiesArgs, Capability},
        Content,
    },
    server::action::ActionError,
};
use log::debug;

pub fn do_get_capabilities(
    respond: impl FnOnce(Content) -> Result<(), ActionError>,
) -> Result<(), ActionError> {
    debug!("do_get_capabilities");
    respond(Content::Capabilities(CapabilitiesArgs {
        capabilities: vec![
            // TODO: Custom
            #[cfg(feature = "exec")]
            Capability::Exec,
            #[cfg(feature = "file-system")]
            Capability::File,
            #[cfg(feature = "forward")]
            Capability::Forward,
        ],
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn do_get_capabilities_should_send_capabilities() {
        let mut content: Option<Content> = None;

        do_get_capabilities(|c| {
            content = Some(c);
            Ok(())
        })
        .unwrap();

        assert_eq!(
            content.unwrap(),
            Content::Capabilities(CapabilitiesArgs {
                capabilities: vec![Capability::Exec, Capability::File, Capability::Forward],
            })
        );
    }
}
