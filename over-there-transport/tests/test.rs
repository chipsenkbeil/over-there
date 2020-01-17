use over_there_auth::Sha256Authenticator;
use over_there_crypto::{self as crypto, aes_gcm};
use over_there_transport::{net, TcpStreamTransceiver, UdpTransceiver};
use over_there_utils::exec;
use std::time::Duration;

fn init() {
    let _ = env_logger::builder()
        .is_test(true)
        .filter_level(log::LevelFilter::Debug)
        .try_init();
}

#[test]
fn test_udp_send_recv() -> Result<(), Box<dyn std::error::Error>> {
    init();
    let encrypt_key = crypto::key::new_256bit_key();
    let sign_key = b"my signature key";

    let mut client = UdpTransceiver::new(
        net::udp::local()?,
        Duration::from_secs(1),
        Sha256Authenticator::new(sign_key),
        aes_gcm::new_aes_256_gcm_bicrypter(&encrypt_key),
    );

    let mut server = UdpTransceiver::new(
        net::udp::local()?,
        Duration::from_secs(1),
        Sha256Authenticator::new(sign_key),
        aes_gcm::new_aes_256_gcm_bicrypter(&encrypt_key),
    );

    // Send message to server
    let msg = b"test message";
    client.send(server.socket.local_addr()?, msg)?;

    // Keep checking until we receive a complete message from the client
    exec::loop_timeout(Duration::from_millis(500), || {
        // A full message has been received, so we process it to verify
        if let (Some(msg), addr) = server.recv()? {
            match msg {
                x if x == b"test message" => server.send(addr, b"test reply")?,
                x => panic!("Unexpected content {:?}", x),
            }
            return Ok(true);
        }
        Ok(false)
    })?;

    // Now wait for client to receive response
    exec::loop_timeout(Duration::from_millis(500), || {
        // A full message has been received, so we process it to verify
        if let (Some(msg), _addr) = client.recv()? {
            match msg {
                x if x == b"test reply" => (),
                x => panic!("Unexpected content {:?}", x),
            }
            return Ok(true);
        }
        Ok(false)
    })?;

    Ok(())
}

#[test]
fn test_tcp_send_recv() -> Result<(), Box<dyn std::error::Error>> {
    init();
    let encrypt_key = crypto::key::new_256bit_key();
    let sign_key = b"my signature key";

    let server_listener = net::tcp::local()?;
    let server_addr = server_listener.local_addr()?;
    let client_stream = std::net::TcpStream::connect(server_addr)?;

    let mut client = TcpStreamTransceiver::new(
        client_stream,
        Duration::from_secs(1),
        Sha256Authenticator::new(sign_key),
        aes_gcm::new_aes_256_gcm_bicrypter(&encrypt_key),
    );

    let (server_stream, _addr) = server_listener.accept()?;
    let mut server = TcpStreamTransceiver::new(
        server_stream,
        Duration::from_secs(1),
        Sha256Authenticator::new(sign_key),
        aes_gcm::new_aes_256_gcm_bicrypter(&encrypt_key),
    );

    // Send message to server
    let msg = b"test message";
    client.send(msg)?;

    // Keep checking until we receive a complete message from the client
    exec::loop_timeout(Duration::from_millis(500), || {
        // A full message has been received, so we process it to verify
        if let Some(msg) = server.recv()? {
            match msg {
                x if x == b"test message" => server.send(b"test reply")?,
                x => panic!("Unexpected content {:?}", x),
            }
            return Ok(true);
        }
        Ok(false)
    })?;

    // Now wait for client to receive response
    exec::loop_timeout(Duration::from_millis(500), || {
        // A full message has been received, so we process it to verify
        if let Some(msg) = client.recv()? {
            match msg {
                x if x == b"test reply" => (),
                x => panic!("Unexpected content {:?}", x),
            }
            return Ok(true);
        }
        Ok(false)
    })?;

    Ok(())
}
