use crate::{msg::content::Content, server::action::ActionError};
use log::debug;
use std::future::Future;

pub async fn heartbeat<F, R>(respond: F) -> Result<(), ActionError>
where
    F: FnOnce(Content) -> R,
    R: Future<Output = Result<(), ActionError>>,
{
    debug!("heartbeat_request");
    respond(Content::Heartbeat).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn heartbeat_should_send_a_heartbeat() {
        let mut content: Option<Content> = None;

        heartbeat(|c| {
            content = Some(c);
            async { Ok(()) }
        })
        .await
        .unwrap();

        assert_eq!(content.unwrap(), Content::Heartbeat);
    }
}
