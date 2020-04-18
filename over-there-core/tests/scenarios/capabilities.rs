use over_there_core::{Capability, ConnectedClient};

pub async fn async_test(mut client: ConnectedClient) {
    let capabilities = client
        .ask_capabilities()
        .await
        .expect("Failed to get capabilities")
        .capabilities;
    assert_eq!(
        capabilities,
        vec![
            Capability::Custom,
            Capability::FileSystem,
            Capability::Exec,
            Capability::Forward,
        ]
    );
}
