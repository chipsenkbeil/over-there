mod scenarios;
mod setup;

use setup::TestMode;

#[tokio::test]
async fn test_tcp_client_ask_version() {
    let test_bench = setup::setup(TestMode::Tcp).await;
    scenarios::version::async_test(test_bench.client).await;
}

#[tokio::test]
async fn test_udp_client_ask_version() {
    let test_bench = setup::setup(TestMode::Udp).await;
    scenarios::version::async_test(test_bench.client).await;
}

#[tokio::test]
async fn test_tcp_client_file_manipulation() {
    let test_bench = setup::setup(TestMode::Tcp).await;
    scenarios::file::async_test(test_bench.client).await;
}

#[tokio::test]
async fn test_udp_client_file_manipulation() {
    let test_bench = setup::setup(TestMode::Udp).await;
    scenarios::file::async_test(test_bench.client).await;
}

#[tokio::test]
async fn test_tcp_client_dir_manipulation() {
    let test_bench = setup::setup(TestMode::Tcp).await;
    scenarios::dir::async_test(test_bench.client).await;
}

#[tokio::test]
async fn test_udp_client_dir_manipulation() {
    let test_bench = setup::setup(TestMode::Udp).await;
    scenarios::dir::async_test(test_bench.client).await;
}

#[tokio::test]
async fn test_tcp_client_remote_process() {
    let test_bench = setup::setup(TestMode::Tcp).await;
    scenarios::proc::async_test(test_bench.client).await;
}

#[tokio::test]
async fn test_udp_client_remote_process() {
    let test_bench = setup::setup(TestMode::Udp).await;
    scenarios::proc::async_test(test_bench.client).await;
}

#[tokio::test]
async fn test_tcp_client_ask_timeout() {
    let test_bench = setup::setup(TestMode::Tcp).await;
    scenarios::ask_timeout::async_test(test_bench.client).await;
}

#[tokio::test]
async fn test_udp_client_ask_timeout() {
    let test_bench = setup::setup(TestMode::Udp).await;
    scenarios::ask_timeout::async_test(test_bench.client).await;
}
