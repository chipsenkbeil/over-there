pub mod crypto;
pub mod msg;
pub mod packet;
pub mod udp;

use msg::Msg;
use std::error::Error;

pub trait Transport {
    fn send(&self, msg: Msg) -> Result<(), Box<dyn Error>>;
    fn recv(&self) -> Result<Option<Msg>, Box<dyn Error>>;
}
