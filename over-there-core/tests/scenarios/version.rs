use over_there_core::ConnectedClient;

pub async fn async_test(mut client: ConnectedClient) {
    let result = client.ask_version().await;
    assert_eq!(result.unwrap(), env!("CARGO_PKG_VERSION").to_string());
}
