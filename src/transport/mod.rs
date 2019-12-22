pub mod data;
pub mod security;
pub mod udp;

use super::msg::Msg;
use std::error::Error;

pub struct MsgAndAddr(pub Msg, pub std::net::SocketAddr);

pub trait Transport {
    /// Sends a provided message
    fn send(&self, msg_and_addr: MsgAndAddr) -> Result<(), Box<dyn Error>>;

    /// Checks for the next incoming message
    fn recv(&self) -> Result<Option<MsgAndAddr>, Box<dyn Error>>;
}
