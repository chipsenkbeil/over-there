mod scenarios;
mod setup;

use futures::executor;
use setup::TestMode;

#[test]
fn test_tcp_client_ask_version() {
    let (client, _server) = setup::setup(TestMode::Tcp);
    executor::block_on(scenarios::version::test(client));
}

#[test]
fn test_udp_client_ask_version() {
    let (client, _server) = setup::setup(TestMode::Udp);
    executor::block_on(scenarios::version::test(client));
}

#[test]
fn test_tcp_client_file_manipulation() {
    let (client, _server) = setup::setup(TestMode::Tcp);
    executor::block_on(scenarios::file::test(client));
}

#[test]
fn test_udp_client_file_manipulation() {
    let (client, _server) = setup::setup(TestMode::Udp);
    executor::block_on(scenarios::file::test(client));
}
