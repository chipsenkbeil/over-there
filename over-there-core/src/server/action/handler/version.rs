use crate::{
    msg::content::{version::VersionArgs, Content},
    server::action::ActionError,
};
use log::debug;
use std::future::Future;

pub async fn do_get_version<F, R>(respond: F) -> Result<(), ActionError>
where
    F: FnOnce(Content) -> R,
    R: Future<Output = Result<(), ActionError>>,
{
    debug!("version_request");
    respond(Content::Version(VersionArgs {
        version: env!("CARGO_PKG_VERSION").to_string(),
    }))
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn do_get_version_should_send_version() {
        let mut content: Option<Content> = None;

        do_get_version(|c| {
            content = Some(c);
            async { Ok(()) }
        })
        .await
        .unwrap();

        assert_eq!(
            content.unwrap(),
            Content::Version(VersionArgs {
                version: env!("CARGO_PKG_VERSION").to_string(),
            })
        );
    }
}
