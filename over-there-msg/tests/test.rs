use over_there_auth::{Sha256Authenticator, Signer, Verifier};
use over_there_crypto::{self as crypto, aes_gcm, Decrypter, Encrypter};
use over_there_msg::{
    Content, Msg, MsgReceiver, MsgTransmitter, TcpMsgReceiver, TcpMsgTransmitter, UdpMsgReceiver,
    UdpMsgTransmitter,
};
use over_there_transport::{tcp, udp, Receiver, Transmitter};
use over_there_utils::exec;
use std::time::Duration;

fn init() {
    let _ = env_logger::builder()
        .is_test(true)
        .filter_level(log::LevelFilter::Debug)
        .try_init();
}

fn new_transmitter<'a, S: Signer, E: Encrypter>(
    transmission_size: usize,
    signer: &'a S,
    encrypter: &'a E,
) -> Transmitter<'a, S, E> {
    Transmitter::new(transmission_size, signer, encrypter)
}

fn new_receiver<'a, V: Verifier, D: Decrypter>(
    transmission_size: usize,
    verifier: &'a V,
    decrypter: &'a D,
) -> Receiver<'a, V, D> {
    Receiver::new(
        transmission_size,
        100,
        Duration::from_secs(60),
        verifier,
        decrypter,
    )
}

#[test]
fn test_udp_send_recv() -> Result<(), Box<dyn std::error::Error>> {
    init();
    let encrypt_key = crypto::key::new_256bit_key();
    let sign_key = b"my signature key";

    let client_socket = udp::local()?;
    let client_auth = Sha256Authenticator::new(sign_key);
    let client_bicrypter = aes_gcm::new_aes_256_gcm_bicrypter(&encrypt_key);
    let ct = new_transmitter(udp::MAX_IPV4_DATAGRAM_SIZE, &client_auth, &client_bicrypter);
    let cmt = MsgTransmitter::new(&ct);
    let client_transmitter = UdpMsgTransmitter::new(&client_socket, &cmt);
    let cr = new_receiver(udp::MAX_IPV4_DATAGRAM_SIZE, &client_auth, &client_bicrypter);
    let cmr = MsgReceiver::new(&cr);
    let client_receiver = UdpMsgReceiver::new(&client_socket, &cmr);

    let server_socket = udp::local()?;
    let server_auth = Sha256Authenticator::new(sign_key);
    let server_bicrypter = aes_gcm::new_aes_256_gcm_bicrypter(&encrypt_key);
    let st = new_transmitter(udp::MAX_IPV4_DATAGRAM_SIZE, &server_auth, &server_bicrypter);
    let smt = MsgTransmitter::new(&st);
    let server_transmitter = UdpMsgTransmitter::new(&server_socket, &smt);
    let sr = new_receiver(udp::MAX_IPV4_DATAGRAM_SIZE, &server_auth, &server_bicrypter);
    let smr = MsgReceiver::new(&sr);
    let server_receiver = UdpMsgReceiver::new(&server_socket, &smr);

    // Send message to server
    let req = Content::HeartbeatRequest;
    let msg = Msg::from(req);
    client_transmitter.send(msg, server_socket.local_addr()?)?;

    // Keep checking until we receive a complete message from the client
    exec::loop_timeout(Duration::from_millis(500), || {
        // A full message has been received, so we process it to verify
        if let Some((msg, addr)) = server_receiver.recv()? {
            match msg.content {
                Content::HeartbeatRequest => {
                    server_transmitter.send(Msg::from(Content::HeartbeatResponse), addr)?
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
        if let Some((msg, _addr)) = client_receiver.recv()? {
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
    let mut client_stream_1 = std::net::TcpStream::connect(server_listener.local_addr()?)?;
    let mut client_stream_2 = client_stream_1.try_clone()?;

    let client_auth = Sha256Authenticator::new(sign_key);
    let client_bicrypter = aes_gcm::new_aes_256_gcm_bicrypter(&encrypt_key);
    let ct = new_transmitter(tcp::MTU_ETHERNET_SIZE, &client_auth, &client_bicrypter);
    let cmt = MsgTransmitter::new(&ct);
    let mut client_transmitter = TcpMsgTransmitter::new(&mut client_stream_1, &cmt);
    let cr = new_receiver(tcp::MTU_ETHERNET_SIZE, &client_auth, &client_bicrypter);
    let cmr = MsgReceiver::new(&cr);
    let mut client_receiver = TcpMsgReceiver::new(&mut client_stream_2, &cmr);

    let mut server_stream_1 = server_listener.accept()?.0;
    let mut server_stream_2 = server_stream_1.try_clone()?;
    let server_auth = Sha256Authenticator::new(sign_key);
    let server_bicrypter = aes_gcm::new_aes_256_gcm_bicrypter(&encrypt_key);
    let st = new_transmitter(tcp::MTU_ETHERNET_SIZE, &server_auth, &server_bicrypter);
    let smt = MsgTransmitter::new(&st);
    let mut server_transmitter = TcpMsgTransmitter::new(&mut server_stream_1, &smt);
    let sr = new_receiver(tcp::MTU_ETHERNET_SIZE, &server_auth, &server_bicrypter);
    let smr = MsgReceiver::new(&sr);
    let mut server_receiver = TcpMsgReceiver::new(&mut server_stream_2, &smr);

    // Send message to server
    let req = Content::HeartbeatRequest;
    let msg = Msg::from(req);
    client_transmitter.send(msg)?;

    // Keep checking until we receive a complete message from the client
    exec::loop_timeout(Duration::from_millis(500), || {
        // A full message has been received, so we process it to verify
        if let Some(msg) = server_receiver.recv()? {
            match msg.content {
                Content::HeartbeatRequest => {
                    server_transmitter.send(Msg::from(Content::HeartbeatResponse))?
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
        if let Some(msg) = client_receiver.recv()? {
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
