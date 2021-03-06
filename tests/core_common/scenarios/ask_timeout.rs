use over_there::core::{
    request::CustomArgs, AskError, ConnectedClient, Request,
};
use std::time::Duration;

pub async fn async_test(mut client: ConnectedClient) {
    // Make the timeout really short so we don't wait too long for the ask
    // to fail
    client.timeout = Duration::from_millis(10);

    // Ask for something custom, which won't have a response; this would
    // cause us to wait forever if we didn't have a timeout
    let result = client
        .ask(From::from(Request::Custom(CustomArgs { data: vec![] })))
        .await;

    assert_eq!(result.unwrap_err(), AskError::Timeout);
}
