extern crate over_there;

use over_there::msg::{Msg, Request, Response};
use over_there::transport::udp::UDP;
use over_there::transport::MsgAndAddr;
use over_there::transport::Transport;

#[test]
fn test_udp_send_recv() -> Result<(), Box<dyn std::error::Error>> {
    let client = UDP::local()?;
    let server = UDP::local()?;

    // Send message to server
    let id = 123;
    let req = Request::HeartbeatRequest;
    let msg = Msg::from_request(id, vec![], req);
    let msg_and_addr = MsgAndAddr(msg, server.addr()?);
    assert_eq!(
        client.send(msg_and_addr).is_ok(),
        true,
        "Failed to send client message"
    );

    // Keep checking until we receive a complete message from the client
    loop {
        // A full message has been received, so we process it to verify
        if let Some(MsgAndAddr(msg, addr)) = server.recv()? {
            match msg.get_request() {
                Some(req) => match req {
                    Request::HeartbeatRequest => {
                        let msg_and_addr = MsgAndAddr(
                            Msg::from_response(id, vec![], Response::HeartbeatResponse),
                            addr,
                        );
                        server.send(msg_and_addr)?
                    }
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
        if let Some(MsgAndAddr(msg, _addr)) = client.recv()? {
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
