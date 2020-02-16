use over_there_auth::Sha256Authenticator;
use over_there_core::{Client, Communicator, Server, Transport};
use over_there_crypto::{self as crypto, aes_gcm};
use over_there_wire::{self as wire, constants};
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
pub const CHANNEL_MAX_SIZE: usize = 1000;

pub async fn setup(mode: TestMode) -> TestBench {
    setup_with_timeout(mode, DEFAULT_TIMEOUT).await
}

pub async fn setup_with_timeout(mode: TestMode, timeout: Duration) -> TestBench {
    init_logger();

    let mut test_bench = match mode {
        TestMode::Tcp => start_tcp_client_and_server().await,
        TestMode::Udp => start_udp_client_and_server().await,
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

async fn start_tcp_client_and_server() -> TestBench {
    let encrypt_key = crypto::key::new_256bit_key();
    let sign_key = b"my signature key";
    let auth = Sha256Authenticator::new(sign_key);
    let bicrypter = aes_gcm::new_aes_256_gcm_bicrypter(&encrypt_key);

    let server = Communicator::new(
        constants::DEFAULT_TTL,
        auth.clone(),
        auth.clone(),
        bicrypter.clone(),
        bicrypter.clone(),
    )
    .listen(
        Transport::Tcp(wire::net::make_local_ipv4_addr_list()),
        CHANNEL_MAX_SIZE,
    )
    .await
    .unwrap();

    let client = Communicator::new(
        constants::DEFAULT_TTL,
        auth.clone(),
        auth.clone(),
        bicrypter.clone(),
        bicrypter.clone(),
    )
    .connect(Transport::Tcp(vec![server.addr()]), CHANNEL_MAX_SIZE)
    .await
    .unwrap();

    TestBench { client, server }
}

async fn start_udp_client_and_server() -> TestBench {
    let encrypt_key = crypto::key::new_256bit_key();
    let sign_key = b"my signature key";
    let auth = Sha256Authenticator::new(sign_key);
    let bicrypter = aes_gcm::new_aes_256_gcm_bicrypter(&encrypt_key);

    let server = Communicator::new(
        constants::DEFAULT_TTL,
        auth.clone(),
        auth.clone(),
        bicrypter.clone(),
        bicrypter.clone(),
    )
    .listen(
        Transport::Udp(wire::net::make_local_ipv4_addr_list()),
        CHANNEL_MAX_SIZE,
    )
    .await
    .unwrap();

    let client = Communicator::new(
        constants::DEFAULT_TTL,
        auth.clone(),
        auth.clone(),
        bicrypter.clone(),
        bicrypter.clone(),
    )
    .connect(Transport::Udp(vec![server.addr()]), CHANNEL_MAX_SIZE)
    .await
    .unwrap();

    TestBench { client, server }
}
