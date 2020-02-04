mod scenarios;
mod setup;

use setup::TestMode;

#[tokio::test]
async fn test_tcp_client_ask_version() {
    let test_bench = setup::setup(TestMode::Tcp);
    scenarios::version::async_test(test_bench.client).await;
}

#[tokio::test]
async fn test_udp_client_ask_version() {
    let test_bench = setup::setup(TestMode::Udp);
    scenarios::version::async_test(test_bench.client).await;
}

#[tokio::test]
async fn test_tcp_client_file_manipulation() {
    let test_bench = setup::setup(TestMode::Tcp);
    scenarios::file::async_test(test_bench.client).await;
}

#[tokio::test]
async fn test_udp_client_file_manipulation() {
    let test_bench = setup::setup(TestMode::Udp);
    scenarios::file::async_test(test_bench.client).await;
}

#[tokio::test]
async fn test_tcp_client_ask_timeout() {
    let test_bench = setup::setup(TestMode::Tcp);
    scenarios::ask_timeout::async_test(test_bench.client).await;
}

#[tokio::test]
async fn test_udp_client_ask_timeout() {
    let test_bench = setup::setup(TestMode::Udp);
    scenarios::ask_timeout::async_test(test_bench.client).await;
}
