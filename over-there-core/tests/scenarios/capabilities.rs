use over_there_core::{Capability, ConnectedClient};

pub async fn async_test(mut client: ConnectedClient) {
    let capabilities = client
        .ask_capabilities()
        .await
        .expect("Failed to get capabilities")
        .capabilities;

    let expected = vec![
        Capability::Custom,
        Capability::FileSystem,
        Capability::Exec,
        Capability::Forward,
    ];

    assert_eq!(
        capabilities.len(),
        expected.len(),
        "Unexpected number of capabilities"
    );

    for c in capabilities.iter() {
        assert!(expected.contains(c), "Missing {:?}", c);
    }
}
