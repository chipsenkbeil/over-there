use log::error;
use over_there_auth::Sha256Authenticator;
use over_there_core::{Client, Server};
use over_there_crypto::{self as crypto, aes_gcm};
use over_there_transport::{constants, net};
use over_there_utils::exec;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

fn init() {
    let _ = env_logger::builder()
        .is_test(true)
        .filter_level(log::LevelFilter::Info)
        .try_init();
}

#[test]
fn test_tcp_version_request_reply() -> Result<(), Box<dyn std::error::Error>> {
    init();
    let encrypt_key = crypto::key::new_256bit_key();
    let sign_key = b"my signature key";

    let server = Server::listen_using_tcp_listener(
        net::tcp::local()?,
        Duration::from_secs(constants::DEFAULT_TTL_IN_SECS),
        Sha256Authenticator::new(sign_key),
        aes_gcm::new_aes_256_gcm_bicrypter(&encrypt_key),
        |e| {
            error!("SERVER {:?}", e);
            false
        },
    )?;

    let client = Client::connect_tcp(
        server.addr,
        Duration::from_secs(constants::DEFAULT_TTL_IN_SECS),
        Sha256Authenticator::new(sign_key),
        aes_gcm::new_aes_256_gcm_bicrypter(&encrypt_key),
        |e| {
            error!("CLIENT {:?}", e);
            false
        },
    )?;

    let version = Arc::new(Mutex::new(String::new()));
    let thread_version = Arc::clone(&version);
    client.ask_version(move |v| *thread_version.lock().unwrap() = v)?;

    // Block until we verify the version
    exec::loop_timeout_panic(Duration::from_millis(2500), || {
        thread::sleep(Duration::from_millis(50));
        let version = version.lock().unwrap().to_string();
        version == env!("CARGO_PKG_VERSION").to_string()
    });

    Ok(())
}

#[test]
fn test_udp_version_request_reply() -> Result<(), Box<dyn std::error::Error>> {
    init();
    let encrypt_key = crypto::key::new_256bit_key();
    let sign_key = b"my signature key";

    let server = Server::listen_using_udp_socket(
        net::udp::local()?,
        Duration::from_secs(constants::DEFAULT_TTL_IN_SECS),
        Sha256Authenticator::new(sign_key),
        aes_gcm::new_aes_256_gcm_bicrypter(&encrypt_key),
        |e| {
            error!("SERVER {:?}", e);
            false
        },
    )?;

    let client = Client::connect_udp(
        server.addr,
        Duration::from_secs(constants::DEFAULT_TTL_IN_SECS),
        Sha256Authenticator::new(sign_key),
        aes_gcm::new_aes_256_gcm_bicrypter(&encrypt_key),
        |e| {
            error!("CLIENT {:?}", e);
            false
        },
    )?;

    let version = Arc::new(Mutex::new(String::new()));
    let thread_version = Arc::clone(&version);
    client.ask_version(move |v| *thread_version.lock().unwrap() = v)?;

    // Block until we verify the version
    exec::loop_timeout_panic(Duration::from_millis(2500), || {
        thread::sleep(Duration::from_millis(50));
        let version = version.lock().unwrap().to_string();
        version == env!("CARGO_PKG_VERSION").to_string()
    });

    Ok(())
}
