use futures::executor;
use log::error;
use over_there_auth::Sha256Authenticator;
use over_there_core::{Client, Server};
use over_there_crypto::{self as crypto, aes_gcm};
use over_there_transport::{constants, net};
use std::time::Duration;

fn init() {
    let _ = env_logger::builder()
        .is_test(true)
        .filter_level(log::LevelFilter::Info)
        .try_init();
}

#[test]
fn test_tcp_client_ask_version() {
    init();
    let encrypt_key = crypto::key::new_256bit_key();
    let sign_key = b"my signature key";

    let server = Server::listen_using_tcp_listener(
        net::tcp::local().unwrap(),
        Duration::from_secs(constants::DEFAULT_TTL_IN_SECS),
        Sha256Authenticator::new(sign_key),
        aes_gcm::new_aes_256_gcm_bicrypter(&encrypt_key),
        |e| {
            error!("SERVER {:?}", e);
            false
        },
    )
    .unwrap();

    let mut client = Client::connect_tcp(
        server.addr,
        Duration::from_secs(constants::DEFAULT_TTL_IN_SECS),
        Sha256Authenticator::new(sign_key),
        aes_gcm::new_aes_256_gcm_bicrypter(&encrypt_key),
        |e| {
            error!("CLIENT {:?}", e);
            false
        },
    )
    .unwrap();

    // Ensure that we fail after 2.5s
    client.timeout = Duration::from_millis(2500);
    let result = executor::block_on(client.ask_version());
    assert_eq!(result.unwrap(), env!("CARGO_PKG_VERSION").to_string());
}

#[test]
fn test_udp_client_ask_version() {
    init();
    let encrypt_key = crypto::key::new_256bit_key();
    let sign_key = b"my signature key";

    let server = Server::listen_using_udp_socket(
        net::udp::local().unwrap(),
        Duration::from_secs(constants::DEFAULT_TTL_IN_SECS),
        Sha256Authenticator::new(sign_key),
        aes_gcm::new_aes_256_gcm_bicrypter(&encrypt_key),
        |e| {
            error!("SERVER {:?}", e);
            false
        },
    )
    .unwrap();

    let mut client = Client::connect_udp(
        server.addr,
        Duration::from_secs(constants::DEFAULT_TTL_IN_SECS),
        Sha256Authenticator::new(sign_key),
        aes_gcm::new_aes_256_gcm_bicrypter(&encrypt_key),
        |e| {
            error!("CLIENT {:?}", e);
            false
        },
    )
    .unwrap();

    // Ensure that we fail after 2.5s
    client.timeout = Duration::from_millis(2500);
    let result = executor::block_on(client.ask_version());
    assert_eq!(result.unwrap(), env!("CARGO_PKG_VERSION").to_string());
}
