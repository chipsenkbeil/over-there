use over_there_auth::Sha256Authenticator;
use over_there_core::{
    ClientBuilder, ConnectedClient, ListeningServer, ServerBuilder, Transport,
};
use over_there_crypto::{self as crypto, Aes256GcmBicrypter};
use over_there_wire::{self as wire};
use std::time::Duration;

pub enum TestMode {
    Tcp,
    Udp,
}

pub struct TestBench {
    pub client: ConnectedClient,
    pub server: ListeningServer,
}

pub const DEFAULT_TIMEOUT: Duration = Duration::from_millis(2500);

pub async fn setup(mode: TestMode) -> TestBench {
    setup_with_timeout(mode, DEFAULT_TIMEOUT).await
}

pub async fn setup_with_timeout(
    mode: TestMode,
    timeout: Duration,
) -> TestBench {
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
    let bicrypter = Aes256GcmBicrypter::new(&encrypt_key);

    let server = ServerBuilder::default()
        .authenticator(auth.clone())
        .bicrypter(bicrypter.clone())
        .transport(Transport::Tcp(wire::net::make_local_ipv4_addr_list()))
        .build()
        .expect("Failed to build server config")
        .cloneable_listen()
        .await
        .expect("Failed to listen");

    let client = ClientBuilder::default()
        .authenticator(auth.clone())
        .bicrypter(bicrypter.clone())
        .transport(Transport::Tcp(vec![server.addr()]))
        .build()
        .expect("Failed to build client config")
        .connect()
        .await
        .expect("Failed to connect");

    TestBench { client, server }
}

async fn start_udp_client_and_server() -> TestBench {
    let encrypt_key = crypto::key::new_256bit_key();
    let sign_key = b"my signature key";
    let auth = Sha256Authenticator::new(sign_key);
    let bicrypter = Aes256GcmBicrypter::new(&encrypt_key);

    let server = ServerBuilder::default()
        .authenticator(auth.clone())
        .bicrypter(bicrypter.clone())
        .transport(Transport::Udp(wire::net::make_local_ipv4_addr_list()))
        .build()
        .expect("Failed to build server config")
        .listen()
        .await
        .expect("Failed to listen");

    let client = ClientBuilder::default()
        .authenticator(auth.clone())
        .bicrypter(bicrypter.clone())
        .transport(Transport::Udp(vec![server.addr()]))
        .build()
        .expect("Failed to build client config")
        .connect()
        .await
        .expect("Failed to connect");

    TestBench { client, server }
}
