use crate::parsers;
use clap::Clap;
use over_there_core::Client;
use std::time::Duration;

#[derive(Clap, Debug)]
pub struct ClientCommand {
    #[clap(long, parse(try_from_str = parsers::parse_duration), default_value = "5")]
    /// Timeout (in seconds) used when communicating with the server
    pub timeout: Duration,
}

pub fn hello() {
    println!("HELLO! {:?}", Client::DEFAULT_TIMEOUT);
}
