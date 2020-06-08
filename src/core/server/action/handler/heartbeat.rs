use log::debug;

pub async fn heartbeat() {
    debug!("heartbeat_request");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn heartbeat_should_do_nothing() {
        heartbeat().await;
    }
}
