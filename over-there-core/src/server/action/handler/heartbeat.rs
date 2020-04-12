use log::debug;

pub async fn heartbeat() {
    debug!("heartbeat_request");
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
