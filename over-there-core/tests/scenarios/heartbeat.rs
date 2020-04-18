use over_there_core::ConnectedClient;

pub async fn async_test(mut client: ConnectedClient) {
    assert!(client.ask_heartbeat().await.is_ok());
}
