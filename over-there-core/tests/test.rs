use over_there_auth::{Sha256Authenticator, Signer, Verifier};
use over_there_crypto::{self as crypto, aes_gcm, Decrypter, Encrypter};
use over_there_msg::{Content, Msg};
use over_there_transport::{tcp, udp, Receiver, Transmitter, TransmitterError};
use over_there_utils::{exec, Capture};
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
    let ct_send = |msg: Msg, addr| -> Result<(), Box<dyn std::error::Error>> {
        let data = msg.to_vec()?;
        let send = udp::new_send_func(client_socket.try_clone()?, addr);
        Ok(ct.send(&data, send)?)
    };
    let cr = new_receiver(udp::MAX_IPV4_DATAGRAM_SIZE, &client_auth, &client_bicrypter);
    let cr_recv = || -> Result<Option<(Msg, _)>, Box<dyn std::error::Error>> {
        let socket = client_socket.try_clone()?;
        let mut recv = udp::new_recv_func(socket);
        let capture = Capture::default();
        let recv = |data: &mut [u8]| {
            let (size, addr) = recv(data)?;
            capture.set(addr);

            let sock = client_socket.try_clone()?;
            let send = |data: &[u8]| -> Result<(), TransmitterError> {
                let send = udp::new_send_func(sock, addr);
                Ok(ct.send(&data, send)?)
            };

            Ok((size, send))
        };
        let (maybe_msg, reply) = cr.recv(recv)?;
        let maybe_msg = maybe_msg.map(|d| Msg::from_slice(&d)).transpose()?;
        Ok(maybe_msg.map(|m| (m, reply)))
    };

    let server_socket = udp::local()?;
    let server_auth = Sha256Authenticator::new(sign_key);
    let server_bicrypter = aes_gcm::new_aes_256_gcm_bicrypter(&encrypt_key);
    let st = new_transmitter(udp::MAX_IPV4_DATAGRAM_SIZE, &server_auth, &server_bicrypter);
    let st_send = |msg: Msg, addr| -> Result<(), Box<dyn std::error::Error>> {
        let data = msg.to_vec()?;
        let send = udp::new_send_func(server_socket.try_clone()?, addr);
        Ok(st.send(&data, send)?)
    };
    let sr = new_receiver(udp::MAX_IPV4_DATAGRAM_SIZE, &server_auth, &server_bicrypter);
    let sr_recv = || -> Result<Option<(Msg, _)>, Box<dyn std::error::Error>> {
        let mut recv = udp::new_recv_func(server_socket.try_clone()?);
        let capture = Capture::default();
        let recv = |data: &mut [u8]| {
            let (size, addr) = recv(data)?;
            capture.set(addr);

            let sock = server_socket.try_clone()?;
            let send = move |data: &[u8]| -> Result<(), TransmitterError> {
                let send = udp::new_send_func(sock, addr);
                Ok(st.send(&data, send)?)
            };

            Ok((size, send))
        };
        let (maybe_msg, reply) = sr.recv(recv)?;
        let maybe_msg = maybe_msg.map(|d| Msg::from_slice(&d)).transpose()?;
        Ok(maybe_msg.map(|m| (m, reply)))
    };

    // Send message to server
    let req = Content::HeartbeatRequest;
    let msg = Msg::from(req);
    ct_send(msg, server_socket.local_addr()?)?;

    // Keep checking until we receive a complete message from the client
    exec::loop_timeout(Duration::from_millis(500), || {
        // A full message has been received, so we process it to verify
        if let Some((msg, reply)) = sr_recv()? {
            match msg.content {
                Content::HeartbeatRequest => {
                    reply(&Msg::from(Content::HeartbeatResponse).to_vec()?)?
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
        if let Some((msg, _addr)) = cr_recv()? {
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
    let server_addr = server_listener.local_addr()?;
    let client_stream = std::net::TcpStream::connect(server_addr)?;

    let client_auth = Sha256Authenticator::new(sign_key);
    let client_bicrypter = aes_gcm::new_aes_256_gcm_bicrypter(&encrypt_key);
    let c_send = tcp::new_send_func(client_stream.try_clone()?);
    let ct = new_transmitter(tcp::MTU_ETHERNET_SIZE, &client_auth, &client_bicrypter);
    let ct_send = |msg: Msg| -> Result<(), Box<dyn std::error::Error>> {
        let data = msg.to_vec()?;
        Ok(ct.send(&data, c_send)?)
    };
    let cr = new_receiver(tcp::MTU_ETHERNET_SIZE, &client_auth, &client_bicrypter);
    let cr_recv = || -> Result<Option<(Msg, _)>, Box<dyn std::error::Error>> {
        let mut recv = tcp::new_recv_func(client_stream.try_clone()?, server_addr);
        let capture = Capture::default();
        let recv = |data: &mut [u8]| {
            let (size, addr) = recv(data)?;
            capture.set(addr);
            Ok((size, c_send))
        };
        let (maybe_msg, reply) = cr.recv(recv)?;
        let maybe_msg = maybe_msg.map(|d| Msg::from_slice(&d)).transpose()?;
        Ok(maybe_msg.map(|m| (m, reply)))
    };

    let server_stream = server_listener.accept()?;
    let server_auth = Sha256Authenticator::new(sign_key);
    let server_bicrypter = aes_gcm::new_aes_256_gcm_bicrypter(&encrypt_key);
    let st = new_transmitter(tcp::MTU_ETHERNET_SIZE, &server_auth, &server_bicrypter);
    let s_send = tcp::new_send_func(server_stream.0.try_clone()?);
    let st_send = |msg: Msg| -> Result<(), Box<dyn std::error::Error>> {
        let data = msg.to_vec()?;
        Ok(st.send(&data, s_send)?)
    };
    let sr = new_receiver(tcp::MTU_ETHERNET_SIZE, &server_auth, &server_bicrypter);
    let sr_recv = || -> Result<Option<(Msg, _)>, Box<dyn std::error::Error>> {
        let mut recv = tcp::new_recv_func(server_stream.0.try_clone()?, server_stream.1);
        let capture = Capture::default();
        let recv = |data: &mut [u8]| {
            let (size, addr) = recv(data)?;
            capture.set(addr);
            Ok((size, s_send))
        };
        let (maybe_msg, reply) = sr.recv(recv)?;
        let maybe_msg = maybe_msg.map(|d| Msg::from_slice(&d)).transpose()?;
        Ok(maybe_msg.map(|m| (m, reply)))
    };

    // Send message to server
    let req = Content::HeartbeatRequest;
    let msg = Msg::from(req);
    ct_send(msg)?;

    // Keep checking until we receive a complete message from the client
    exec::loop_timeout(Duration::from_millis(500), || {
        // A full message has been received, so we process it to verify
        if let Some((msg, _addr)) = sr_recv()? {
            match msg.content {
                Content::HeartbeatRequest => st_send(Msg::from(Content::HeartbeatResponse))?,
                x => panic!("Unexpected content {:?}", x),
            }
            return Ok(true);
        }
        Ok(false)
    })?;

    // Now wait for client to receive response
    exec::loop_timeout(Duration::from_millis(500), || {
        // A full message has been received, so we process it to verify
        if let Some((msg, _addr)) = cr_recv()? {
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
