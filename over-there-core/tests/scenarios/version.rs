use over_there_core::Client;
use std::time::Duration;

pub async fn async_test(mut client: Client) {
    // Ensure that we fail after 2.5s
    client.timeout = Duration::from_millis(2500);

    let result = client.ask_version().await;
    assert_eq!(result.unwrap(), env!("CARGO_PKG_VERSION").to_string());
}
