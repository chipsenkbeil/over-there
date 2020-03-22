use crate::{
    msg::content::{CapabilitiesArgs, Capability, Content},
    server::action::ActionError,
};
use log::debug;
use std::future::Future;

pub async fn do_get_capabilities<F, R>(respond: F) -> Result<(), ActionError>
where
    F: FnOnce(Content) -> R,
    R: Future<Output = Result<(), ActionError>>,
{
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
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn do_get_capabilities_should_send_capabilities() {
        let mut content: Option<Content> = None;

        do_get_capabilities(|c| {
            content = Some(c);
            async { Ok(()) }
        })
        .await
        .unwrap();

        assert_eq!(
            content.unwrap(),
            Content::Capabilities(CapabilitiesArgs {
                capabilities: vec![
                    Capability::Exec,
                    Capability::File,
                    Capability::Forward
                ],
            })
        );
    }
}
