use over_there_core::Client;

pub async fn async_test(mut client: Client) {
    let result = client.ask_version().await;
    assert_eq!(result.unwrap(), env!("CARGO_PKG_VERSION").to_string());
}
