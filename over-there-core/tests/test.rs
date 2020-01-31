mod scenarios;
mod setup;

use futures::executor;
use setup::TestMode;

#[test]
fn test_tcp_client_ask_version() {
    let test_bench = setup::setup(TestMode::Tcp);
    executor::block_on(scenarios::version::async_test(test_bench.client));
}

#[test]
fn test_udp_client_ask_version() {
    let test_bench = setup::setup(TestMode::Udp);
    executor::block_on(scenarios::version::async_test(test_bench.client));
}

#[test]
fn test_tcp_client_file_manipulation() {
    let test_bench = setup::setup(TestMode::Tcp);
    executor::block_on(scenarios::file::async_test(test_bench.client));
}

#[test]
fn test_udp_client_file_manipulation() {
    let test_bench = setup::setup(TestMode::Udp);
    executor::block_on(scenarios::file::async_test(test_bench.client));
}
