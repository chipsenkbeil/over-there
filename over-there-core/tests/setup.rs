use log::error;
use over_there_auth::Sha256Authenticator;
use over_there_core::{Client, Server};
use over_there_crypto::{self as crypto, aes_gcm};
use over_there_transport::{constants, net};
use std::time::Duration;

pub enum TestMode {
    Tcp,
    Udp,
}

pub struct TestBench {
    pub client: Client,
    pub server: Server,
}

pub const DEFAULT_TIMEOUT: Duration = Duration::from_millis(2500);

pub fn setup(mode: TestMode) -> TestBench {
    setup_with_timeout(mode, DEFAULT_TIMEOUT)
}

pub fn setup_with_timeout(mode: TestMode, timeout: Duration) -> TestBench {
    init_logger();

    let mut test_bench = match mode {
        TestMode::Tcp => start_tcp_client_and_server(),
        TestMode::Udp => start_udp_client_and_server(),
    };

    // Ensure that we fail after the provided timeout
    test_bench.client.timeout = timeout;

    test_bench
}

fn init_logger() {
    let _ = env_logger::builder()
        .is_test(true)
        .filter_level(log::LevelFilter::Info)
        .try_init();
}

fn start_tcp_client_and_server() -> TestBench {
    let encrypt_key = crypto::key::new_256bit_key();
    let sign_key = b"my signature key";

    let server = Server::listen_using_tcp_listener(
        net::tcp::local().unwrap(),
        Duration::from_secs(constants::DEFAULT_TTL_IN_SECS),
        Sha256Authenticator::new(sign_key),
        aes_gcm::new_aes_256_gcm_bicrypter(&encrypt_key),
        |e| {
            error!("TCP SERVER {:?}", e);
            false
        },
    )
    .unwrap();

    let client = Client::connect_tcp(
        server.addr,
        Duration::from_secs(constants::DEFAULT_TTL_IN_SECS),
        Sha256Authenticator::new(sign_key),
        aes_gcm::new_aes_256_gcm_bicrypter(&encrypt_key),
        |e| {
            error!("TCP CLIENT {:?}", e);
            false
        },
    )
    .unwrap();

    TestBench { client, server }
}

fn start_udp_client_and_server() -> TestBench {
    let encrypt_key = crypto::key::new_256bit_key();
    let sign_key = b"my signature key";

    let server = Server::listen_using_udp_socket(
        net::udp::local().unwrap(),
        Duration::from_secs(constants::DEFAULT_TTL_IN_SECS),
        Sha256Authenticator::new(sign_key),
        aes_gcm::new_aes_256_gcm_bicrypter(&encrypt_key),
        |e| {
            error!("UDP SERVER {:?}", e);
            false
        },
    )
    .unwrap();

    let client = Client::connect_udp(
        server.addr,
        Duration::from_secs(constants::DEFAULT_TTL_IN_SECS),
        Sha256Authenticator::new(sign_key),
        aes_gcm::new_aes_256_gcm_bicrypter(&encrypt_key),
        |e| {
            error!("UDP CLIENT {:?}", e);
            false
        },
    )
    .unwrap();

    TestBench { client, server }
}
