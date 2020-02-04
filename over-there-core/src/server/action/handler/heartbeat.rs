use crate::{msg::content::Content, server::action::ActionError};
use log::debug;

pub async fn heartbeat(
    respond: impl FnOnce(Content) -> Result<(), ActionError>,
) -> Result<(), ActionError> {
    debug!("heartbeat_request");
    respond(Content::Heartbeat)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn heartbeat_should_send_a_heartbeat() {
        let mut content: Option<Content> = None;

        heartbeat(|c| {
            content = Some(c);
            Ok(())
        })
        .await
        .unwrap();

        assert_eq!(content.unwrap(), Content::Heartbeat);
    }
}
