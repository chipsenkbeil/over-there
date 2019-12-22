extern crate over_there;

use over_there::msg::{Msg, Request, Response};
use over_there::transport::udp::UDP;
use over_there::transport::MsgAndAddr;
use over_there::transport::Transport;

#[test]
fn test_udp_send_recv() -> Result<(), Box<dyn std::error::Error>> {
    let client = UDP::local()?;
    let server = UDP::local()?;

    let long_text = "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Cum sociis natoque penatibus et magnis dis. Condimentum vitae sapien pellentesque habitant. Faucibus vitae aliquet nec ullamcorper sit amet. Fermentum leo vel orci porta non pulvinar neque. Tincidunt arcu non sodales neque sodales. Orci eu lobortis elementum nibh tellus molestie. Urna id volutpat lacus laoreet non. Sit amet luctus venenatis lectus magna fringilla urna porttitor rhoncus. Fermentum iaculis eu non diam. Lacus vestibulum sed arcu non. Urna duis convallis convallis tellus id interdum velit laoreet id. Sagittis nisl rhoncus mattis rhoncus. Imperdiet proin fermentum leo vel orci porta non pulvinar neque.
Eleifend mi in nulla posuere sollicitudin aliquam. Sed euismod nisi porta lorem mollis aliquam. Pellentesque elit ullamcorper dignissim cras tincidunt lobortis feugiat vivamus. Ut faucibus pulvinar elementum integer enim neque. Enim sed faucibus turpis in eu mi bibendum neque. Lectus nulla at volutpat diam. Lacinia quis vel eros donec ac odio tempor. Quis risus sed vulputate odio ut enim blandit. Dictum sit amet justo donec enim diam vulputate ut. Facilisis volutpat est velit egestas dui id ornare arcu odio. Pulvinar etiam non quam lacus suspendisse. At volutpat diam ut venenatis tellus in metus. Enim facilisis gravida neque convallis a. Facilisi morbi tempus iaculis urna id volutpat. Odio euismod lacinia at quis risus sed vulputate. Quis commodo odio aenean sed. Accumsan sit amet nulla facilisi morbi tempus iaculis urna.
Lorem mollis aliquam ut porttitor. Sagittis vitae et leo duis. Amet mauris commodo quis imperdiet massa. Massa eget egestas purus viverra accumsan in nisl. Enim tortor at auctor urna nunc id cursus metus. Aliquam purus sit amet luctus venenatis lectus magna. Enim eu turpis egestas pretium aenean pharetra magna ac. Dignissim diam quis enim lobortis scelerisque. Facilisi nullam vehicula ipsum a arcu cursus vitae congue mauris. Sit amet est placerat in egestas erat imperdiet sed. Pretium quam vulputate dignissim suspendisse. Venenatis lectus magna fringilla urna porttitor rhoncus dolor. Sit amet aliquam id diam maecenas ultricies mi eget mauris. Integer eget aliquet nibh praesent. Ipsum a arcu cursus vitae congue mauris. Libero enim sed faucibus turpis in eu mi bibendum neque. Massa sed elementum tempus egestas sed. Duis at tellus at urna condimentum mattis pellentesque id.
Ornare quam viverra orci sagittis eu volutpat odio facilisis mauris. Pulvinar etiam non quam lacus. Cursus eget nunc scelerisque viverra mauris in. Amet nulla facilisi morbi tempus iaculis urna. Felis imperdiet proin fermentum leo vel orci porta non pulvinar. Et netus et malesuada fames ac turpis egestas. Dignissim convallis aenean et tortor at risus. Nulla pellentesque dignissim enim sit amet venenatis. Sit amet est placerat in egestas erat imperdiet. Fermentum odio eu feugiat pretium nibh ipsum. Cursus metus aliquam eleifend mi in nulla posuere sollicitudin aliquam. Venenatis lectus magna fringilla urna porttitor rhoncus dolor.
Mattis vulputate enim nulla aliquet porttitor. A erat nam at lectus urna. Fermentum dui faucibus in ornare quam viverra. Commodo quis imperdiet massa tincidunt nunc. Proin fermentum leo vel orci. Ipsum consequat nisl vel pretium lectus quam. Suspendisse ultrices gravida dictum fusce ut placerat orci nulla. Diam volutpat commodo sed egestas egestas fringilla. Eget est lorem ipsum dolor sit amet. Egestas sed sed risus pretium quam vulputate dignissim. Nunc sed augue lacus viverra vitae. Rutrum tellus pellentesque eu tincidunt tortor aliquam nulla facilisi cras. In mollis nunc sed id semper risus. Elit pellentesque habitant morbi tristique senectus.";

    // Send message to server
    let id = 123;
    let req = Request::ListFilesRequest(String::from(long_text));
    let msg = Msg::from_request(id, vec![], req);
    let msg_and_addr = MsgAndAddr(msg, server.addr()?);
    client.send(msg_and_addr)?;

    // Keep checking until we receive a complete message from the client
    loop {
        // A full message has been received, so we process it to verify
        if let Some(MsgAndAddr(msg, addr)) = server.recv()? {
            match msg.get_request() {
                Some(req) => match req {
                    Request::ListFilesRequest(text) => {
                        assert_eq!(long_text, text, "Received text did not match");
                        println!("GOT LIST FILES REQUEST {:?}", req);
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
