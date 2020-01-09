use over_there_crypto::{self as crypto, aes_gcm, Bicrypter};
use over_there_msg::{Content, Msg, MsgTransmitter, TcpMsgTransmitter, UdpMsgTransmitter};
use over_there_sign::{Authenticator, Sha256Authenticator};
use over_there_transport::{tcp, udp, Transmitter};
use over_there_utils::exec;
use std::time::Duration;

fn init() {
    let _ = env_logger::builder()
        .is_test(true)
        .filter_level(log::LevelFilter::Debug)
        .try_init();
}

fn new_msg_transmitter<A: Authenticator, B: Bicrypter>(
    transmission_size: usize,
    authenticator: A,
    bicrypter: B,
) -> MsgTransmitter<A, B> {
    MsgTransmitter::new(Transmitter::new(
        transmission_size,
        1500,
        Duration::from_secs(5 * 60),
        authenticator,
        bicrypter,
    ))
}

#[test]
fn test_udp_send_recv() -> Result<(), Box<dyn std::error::Error>> {
    init();
    let encrypt_key = crypto::key::new_256bit_key();
    let sign_key = b"my signature key";

    let client = UdpMsgTransmitter::new(
        udp::local()?,
        new_msg_transmitter(
            udp::MAX_IPV4_DATAGRAM_SIZE,
            Sha256Authenticator::new(sign_key),
            aes_gcm::new_aes_256_gcm_bicrypter(&encrypt_key),
        ),
    );
    let server = UdpMsgTransmitter::new(
        udp::local()?,
        new_msg_transmitter(
            udp::MAX_IPV4_DATAGRAM_SIZE,
            Sha256Authenticator::new(sign_key),
            aes_gcm::new_aes_256_gcm_bicrypter(&encrypt_key),
        ),
    );

    // Send message to server
    let req = Content::HeartbeatRequest;
    let msg = Msg::from(req);
    client.send(msg, server.socket.local_addr()?)?;

    // Keep checking until we receive a complete message from the client
    exec::loop_timeout(Duration::from_millis(500), || {
        // A full message has been received, so we process it to verify
        if let Some((msg, addr)) = server.recv()? {
            match msg.content {
                Content::HeartbeatRequest => {
                    server.send(Msg::from(Content::HeartbeatResponse), addr)?
                }
                x => panic!("Unexpected content {:?}", x),
            }
            return Ok(true);
        }
        Ok(false)
    })?;

    // Now wait for client to receive response
    exec::loop_timeout(Duration::from_millis(500), || {
        // A full message has been received, so we process it to verify
        if let Some((msg, _addr)) = client.recv()? {
            match msg.content {
                Content::HeartbeatResponse => (),
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

    let server_listener = tcp::local()?;
    let client_stream = std::net::TcpStream::connect(server_listener.local_addr()?)?;

    let mut client = TcpMsgTransmitter::new(
        client_stream,
        new_msg_transmitter(
            tcp::MTU_ETHERNET_SIZE,
            Sha256Authenticator::new(sign_key),
            aes_gcm::new_aes_256_gcm_bicrypter(&encrypt_key),
        ),
    );
    let mut server = TcpMsgTransmitter::new(
        server_listener.accept()?.0,
        new_msg_transmitter(
            tcp::MTU_ETHERNET_SIZE,
            Sha256Authenticator::new(sign_key),
            aes_gcm::new_aes_256_gcm_bicrypter(&encrypt_key),
        ),
    );

    // Send message to server
    let req = Content::HeartbeatRequest;
    let msg = Msg::from(req);
    client.send(msg)?;

    // Keep checking until we receive a complete message from the client
    exec::loop_timeout(Duration::from_millis(500), || {
        // A full message has been received, so we process it to verify
        if let Some(msg) = server.recv()? {
            match msg.content {
                Content::HeartbeatRequest => server.send(Msg::from(Content::HeartbeatResponse))?,
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
            match msg.content {
                Content::HeartbeatResponse => (),
                x => panic!("Unexpected content {:?}", x),
            }
            return Ok(true);
        }
        Ok(false)
    })?;

    Ok(())
}
