use super::{Msg, Request, Response};
use std::error::Error;

pub fn handle_msg(
    msg: Msg,
    send: &dyn Fn(Msg) -> Result<(), Box<dyn Error>>,
) -> Result<(), Box<dyn Error>> {
    if msg.is_request() {
        handle_request(msg.get_request().unwrap(), send)
    } else if msg.is_response() {
        handle_response(msg.get_response().unwrap(), send)
    } else {
        Ok(())
    }
}

fn handle_request(
    request: &Request,
    _send: &dyn Fn(Msg) -> Result<(), Box<dyn Error>>,
) -> Result<(), Box<dyn Error>> {
    match request {
        Request::HeartbeatRequest => Ok(()),
        _ => Ok(()),
    }
}

fn handle_response(
    response: &Response,
    _send: &dyn Fn(Msg) -> Result<(), Box<dyn Error>>,
) -> Result<(), Box<dyn Error>> {
    match response {
        Response::HeartbeatResponse => Ok(()),
        _ => Ok(()),
    }
}
