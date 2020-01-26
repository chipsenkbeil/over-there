use log::error;
use over_there_auth::Sha256Authenticator;
use over_there_crypto::{self as crypto, aes_gcm};
use over_there_transport::{
    net, NetListener, NetStream, NetTransmission, Responder, TcpListenerTransceiver,
    TcpStreamTransceiver, TransceiverContext, UdpTransceiver,
};
use over_there_utils::exec;
use std::io;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

fn init() {
    let _ = env_logger::builder()
        .is_test(true)
        .filter_level(log::LevelFilter::Debug)
        .try_init();
}

#[test]
fn test_udp_send_recv_single_thread() -> Result<(), Box<dyn std::error::Error>> {
    init();
    let encrypt_key = crypto::key::new_256bit_key();
    let sign_key = b"my signature key";

    let ctx = TransceiverContext::new(
        NetTransmission::UdpIpv4.into(),
        Duration::from_secs(1),
        Sha256Authenticator::new(sign_key),
        aes_gcm::new_aes_256_gcm_bicrypter(&encrypt_key),
    );
    let mut client = UdpTransceiver::new(net::udp::local()?, ctx);

    let ctx = TransceiverContext::new(
        NetTransmission::UdpIpv4.into(),
        Duration::from_secs(1),
        Sha256Authenticator::new(sign_key),
        aes_gcm::new_aes_256_gcm_bicrypter(&encrypt_key),
    );
    let mut server = UdpTransceiver::new(net::udp::local()?, ctx);

    // Send message to server
    let msg = b"test message";
    client.send(server.socket.local_addr()?, msg)?;

    // Keep checking until we receive a complete message from the client
    exec::loop_timeout(Duration::from_millis(500), || {
        // A full message has been received, so we process it to verify
        if let Some((msg, addr)) = server.recv()? {
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
        if let Some((msg, _addr)) = client.recv()? {
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
fn test_udp_send_recv_multi_thread() -> Result<(), Box<dyn std::error::Error>> {
    init();
    let encrypt_key = crypto::key::new_256bit_key();
    let sign_key = b"my signature key";

    let ctx = TransceiverContext::new(
        NetTransmission::UdpIpv4.into(),
        Duration::from_secs(1),
        Sha256Authenticator::new(sign_key),
        aes_gcm::new_aes_256_gcm_bicrypter(&encrypt_key),
    );
    let client = UdpTransceiver::new(net::udp::local()?, ctx);

    let ctx = TransceiverContext::new(
        NetTransmission::UdpIpv4.into(),
        Duration::from_secs(1),
        Sha256Authenticator::new(sign_key),
        aes_gcm::new_aes_256_gcm_bicrypter(&encrypt_key),
    );
    let server = UdpTransceiver::new(net::udp::local()?, ctx);

    let mc_1 = Arc::new(Mutex::new(0));
    let mc_2 = Arc::clone(&mc_1);
    let mc_3 = Arc::clone(&mc_1);
    let rc_1 = Arc::new(Mutex::new(0));
    let rc_2 = Arc::clone(&rc_1);
    let rc_3 = Arc::clone(&rc_1);

    let server_addr = server.socket.local_addr()?;

    // TODO: Must keep in scope to not lose thread? Does not seem to be
    //       an issue with UDP but does happen with TCP. Is there something
    //       else going on here?
    let _server_thread = server.spawn(
        Duration::from_millis(1),
        move |msg, s| {
            let msg = String::from_utf8(msg).unwrap();

            if !msg.starts_with("test message") {
                panic!("Unexpected content {:?}", msg);
            }

            let msg = format!("reply {}", msg);
            s.send(msg.as_bytes()).unwrap();
            *mc_1.lock().unwrap() += 1;
        },
        |e| {
            error!("{:?}", e);
            true
        },
    );

    let client_thread = client.spawn(
        Duration::from_millis(1),
        move |msg, _addr| {
            let msg = String::from_utf8(msg).unwrap();

            if !msg.starts_with("reply") {
                panic!("Unexpected content {:?}", msg);
            }

            *rc_1.lock().unwrap() += 1;
        },
        |e| {
            error!("{:?}", e);
            true
        },
    )?;

    // Send N messages to server
    const N: usize = 7;
    for i in 0..N {
        client_thread.send((
            format!("test message {}", i).as_bytes().to_vec(),
            server_addr,
        ))?;
    }

    // Block until we verify the counts
    exec::loop_timeout(Duration::from_millis(2500), || {
        thread::sleep(Duration::from_millis(50));
        let tmc = *mc_2.lock().unwrap() == N;
        let trc = *rc_2.lock().unwrap() == N;
        Ok(tmc && trc)
    })
    .map_err(|_| {
        io::Error::new(
            io::ErrorKind::TimedOut,
            format!(
                "Client Received: {}/{} | Server Received: {}/{}",
                rc_3.lock().unwrap(),
                N,
                mc_3.lock().unwrap(),
                N
            ),
        )
    })?;

    Ok(())
}

#[test]
fn test_tcp_send_recv_single_thread() -> Result<(), Box<dyn std::error::Error>> {
    init();
    let encrypt_key = crypto::key::new_256bit_key();
    let sign_key = b"my signature key";

    let server_listener = net::tcp::local()?;
    let server_addr = server_listener.local_addr()?;
    let client_stream = std::net::TcpStream::connect(server_addr)?;

    let ctx = TransceiverContext::new(
        NetTransmission::TcpEthernet.into(),
        Duration::from_secs(1),
        Sha256Authenticator::new(sign_key),
        aes_gcm::new_aes_256_gcm_bicrypter(&encrypt_key),
    );
    let mut client = TcpStreamTransceiver::new(client_stream, ctx)?;

    let ctx = TransceiverContext::new(
        NetTransmission::TcpEthernet.into(),
        Duration::from_secs(1),
        Sha256Authenticator::new(sign_key),
        aes_gcm::new_aes_256_gcm_bicrypter(&encrypt_key),
    );
    let (server_stream, _addr) = server_listener.accept()?;
    let mut server = TcpStreamTransceiver::new(server_stream, ctx)?;

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

#[test]
fn test_tcp_send_recv_multi_thread() -> Result<(), Box<dyn std::error::Error>> {
    init();
    let encrypt_key = crypto::key::new_256bit_key();
    let sign_key = b"my signature key";

    let server_listener = net::tcp::local()?;
    let server_addr = server_listener.local_addr()?;
    let client_stream = std::net::TcpStream::connect(server_addr)?;

    let ctx = TransceiverContext::new(
        NetTransmission::TcpEthernet.into(),
        Duration::from_secs(1),
        Sha256Authenticator::new(sign_key),
        crypto::NoopBicrypter, // aes_gcm::new_aes_256_gcm_bicrypter(&encrypt_key),
    );
    let client = TcpStreamTransceiver::new(client_stream, ctx)?;

    let ctx = TransceiverContext::new(
        NetTransmission::TcpEthernet.into(),
        Duration::from_secs(1),
        Sha256Authenticator::new(sign_key),
        crypto::NoopBicrypter, // aes_gcm::new_aes_256_gcm_bicrypter(&encrypt_key),
    );
    let server = TcpListenerTransceiver::new(server_listener, ctx);

    let mc_1 = Arc::new(Mutex::new(0));
    let mc_2 = Arc::clone(&mc_1);
    let mc_3 = Arc::clone(&mc_1);
    let rc_1 = Arc::new(Mutex::new(0));
    let rc_2 = Arc::clone(&rc_1);
    let rc_3 = Arc::clone(&rc_1);

    // NOTE: Must keep in scope otherwise connection is dropped
    let _server_thread = server.spawn(
        Duration::from_millis(1),
        move |msg, s| {
            let msg = String::from_utf8(msg).unwrap();

            if !msg.starts_with("test message") {
                panic!("Unexpected content {:?}", msg);
            }

            let msg = format!("reply {}", msg);
            s.send(msg.as_bytes()).unwrap();
            *mc_1.lock().unwrap() += 1;
        },
        |e| {
            error!("{:?}", e);
            true
        },
    )?;

    // client.send(&vec![0, 1, 2])?;
    let client_thread = client.spawn(
        Duration::from_millis(1),
        move |msg, _send| {
            let msg = String::from_utf8(msg).unwrap();

            if !msg.starts_with("reply") {
                panic!("Unexpected content {:?}", msg);
            }

            *rc_1.lock().unwrap() += 1;
        },
        |e| {
            error!("{:?}", e);
            true
        },
    )?;

    // Send N messages to server
    const N: usize = 7;
    for i in 0..N {
        client_thread.send(format!("test message {}", i).as_bytes().to_vec())?;
    }

    // Block until we verify the counts
    exec::loop_timeout(Duration::from_millis(2500), || {
        thread::sleep(Duration::from_millis(50));
        let mc = *mc_2.lock().unwrap();
        let rc = *rc_2.lock().unwrap();
        Ok(mc == N && rc == N)
    })
    .map_err(|_| {
        io::Error::new(
            io::ErrorKind::TimedOut,
            format!(
                "Client Received: {}/{} | Server Received: {}/{}",
                rc_3.lock().unwrap(),
                N,
                mc_3.lock().unwrap(),
                N
            ),
        )
    })?;

    Ok(())
}
