use over_there_msg::{
    FileMsgTransmitter, Msg, StandardRequest as Request, StandardResponse as Response,
    TcpMsgTransmitter, UdpMsgTransmitter,
};
use over_there_transport::{tcp, udp};
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

    let client = UdpMsgTransmitter::from_socket(udp::local()?);
    let server = UdpMsgTransmitter::from_socket(udp::local()?);

    // Send message to server
    let req = Request::HeartbeatRequest;
    let msg = Msg::from_content(req);
    client.send(msg, server.socket.local_addr()?)?;

    // Keep checking until we receive a complete message from the client
    exec::loop_timeout(Duration::from_millis(500), || {
        // A full message has been received, so we process it to verify
        if let Some((msg, addr)) = server.recv()? {
            if msg.is_content::<Request>() {
                match msg.to_content::<Request>().unwrap() {
                    Request::HeartbeatRequest => {
                        server.send(Msg::from_content(Response::HeartbeatResponse), addr)?
                    }
                    x => panic!("Unexpected request {:?}", x),
                }
            } else {
                panic!("Unexpected msg {:?}", msg);
            }
            return Ok(true);
        }
        Ok(false)
    })?;

    // Now wait for client to receive response
    exec::loop_timeout(Duration::from_millis(500), || {
        // A full message has been received, so we process it to verify
        if let Some((msg, _addr)) = client.recv()? {
            if msg.is_content::<Response>() {
                match msg.to_content::<Response>().unwrap() {
                    Response::HeartbeatResponse => (),
                    x => panic!("Unexpected response {:?}", x),
                }
            } else {
                panic!("Unexpected msg {:?}", msg);
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

    let server_listener = tcp::local()?;
    let client_stream = std::net::TcpStream::connect(server_listener.local_addr()?)?;

    let mut client = TcpMsgTransmitter::from_stream(client_stream);
    let mut server = TcpMsgTransmitter::from_stream(server_listener.accept()?.0);

    // Send message to server
    let req = Request::HeartbeatRequest;
    let msg = Msg::from_content(req);
    client.send(msg)?;

    // Keep checking until we receive a complete message from the client
    exec::loop_timeout(Duration::from_millis(500), || {
        // A full message has been received, so we process it to verify
        if let Some(msg) = server.recv()? {
            if msg.is_content::<Request>() {
                match msg.to_content::<Request>().unwrap() {
                    Request::HeartbeatRequest => {
                        server.send(Msg::from_content(Response::HeartbeatResponse))?
                    }
                    x => panic!("Unexpected request {:?}", x),
                }
            } else {
                panic!("Unexpected msg {:?}", msg);
            }
            return Ok(true);
        }
        Ok(false)
    })?;

    // Now wait for client to receive response
    exec::loop_timeout(Duration::from_millis(500), || {
        // A full message has been received, so we process it to verify
        if let Some(msg) = client.recv()? {
            if msg.is_content::<Response>() {
                match msg.to_content::<Response>().unwrap() {
                    Response::HeartbeatResponse => (),
                    x => panic!("Unexpected response {:?}", x),
                }
            } else {
                panic!("Unexpected msg {:?}", msg);
            }
            return Ok(true);
        }
        Ok(false)
    })?;

    Ok(())
}

#[test]
fn test_file_send_recv() -> Result<(), Box<dyn std::error::Error>> {
    init();

    let client_file = tempfile::tempfile()?;
    let server_file = tempfile::tempfile()?;
    let mut client =
        FileMsgTransmitter::from_files(client_file.try_clone()?, server_file.try_clone()?);
    let mut server = FileMsgTransmitter::from_files(server_file, client_file);

    // Send message to server
    let req = Request::HeartbeatRequest;
    let msg = Msg::from_content(req);
    client.send(msg)?;

    // Keep checking until we receive a complete message from the client
    exec::loop_timeout(Duration::from_millis(500), || {
        // A full message has been received, so we process it to verify
        if let Some(msg) = server.recv()? {
            if msg.is_content::<Request>() {
                match msg.to_content::<Request>().unwrap() {
                    Request::HeartbeatRequest => {
                        server.send(Msg::from_content(Response::HeartbeatResponse))?
                    }
                    x => panic!("Unexpected request {:?}", x),
                }
            } else {
                panic!("Unexpected msg {:?}", msg);
            }
            return Ok(true);
        }
        Ok(false)
    })?;

    // Now wait for client to receive response
    exec::loop_timeout(Duration::from_millis(500), || {
        // A full message has been received, so we process it to verify
        if let Some(msg) = client.recv()? {
            if msg.is_content::<Response>() {
                match msg.to_content::<Response>().unwrap() {
                    Response::HeartbeatResponse => (),
                    x => panic!("Unexpected response {:?}", x),
                }
            } else {
                panic!("Unexpected msg {:?}", msg);
            }
            return Ok(true);
        }
        Ok(false)
    })?;

    Ok(())
}
