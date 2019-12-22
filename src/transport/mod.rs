pub mod data;
pub mod security;
pub mod udp;

use super::msg::Msg;
use std::error::Error;

pub trait Transport {
    /// Sends a provided message
    fn send(&self, msg: Msg) -> Result<(), Box<dyn Error>>;

    /// Checks for the next incoming message
    fn recv(&self) -> Result<Option<Msg>, Box<dyn Error>>;
}
