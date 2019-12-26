extern crate over_there;

use over_there::Communicator;
use over_there::NetworkTransport;
use over_there::UDP;
use over_there::{Msg, Request, Response};

fn init() {
    let _ = env_logger::builder()
        .is_test(true)
        .filter_level(log::LevelFilter::Debug)
        .try_init();
}

#[test]
fn test_udp_send_recv() -> Result<(), Box<dyn std::error::Error>> {
    init();

    let client = Communicator::from_transport(UDP::local()?, UDP::MAX_IPV4_DATAGRAM_SIZE as u32);
    let server = Communicator::from_transport(UDP::local()?, UDP::MAX_IPV4_DATAGRAM_SIZE as u32);

    // Send message to server
    let id = 123;
    let req = Request::HeartbeatRequest;
    let msg = Msg::from_request(id, vec![], req);
    client.send(msg, server.transport().addr()?)?;

    // Keep checking until we receive a complete message from the client
    loop {
        // A full message has been received, so we process it to verify
        if let Some((msg, addr)) = server.recv()? {
            match msg.get_request() {
                Some(req) => match req {
                    Request::HeartbeatRequest => server.send(
                        Msg::from_response(id, vec![], Response::HeartbeatResponse),
                        addr,
                    )?,
                    _ => panic!("Unexpected request {:?}", req),
                },
                _ => panic!("Unexpected message {:?}", msg),
            }
            break;
        }
    }

    // Now wait for client to receive response
    loop {
        // A full message has been received, so we process it to verify
        if let Some((msg, _addr)) = client.recv()? {
            match msg.get_response() {
                Some(res) => match res {
                    Response::HeartbeatResponse => (),
                    _ => panic!("Unexpected response {:?}", res),
                },
                _ => panic!("Unexpected message {:?}", msg),
            }
            break;
        }
    }

    Ok(())
}
