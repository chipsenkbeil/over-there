use over_there_core::ConnectedClient;

pub async fn async_test(mut client: ConnectedClient) {
    let version = client
        .ask_version()
        .await
        .expect("Failed to get version")
        .version;
    assert_eq!(version, env!("CARGO_PKG_VERSION").to_string());
}
