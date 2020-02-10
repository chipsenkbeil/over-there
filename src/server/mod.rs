use crate::parsers;
use clap::Clap;
use std::time::Duration;

#[derive(Clap, Debug)]
pub struct ServerCommand {
    #[clap(long)]
    pub name: String,

    #[clap(long, parse(try_from_str = parsers::parse_duration), default_value = "5")]
    /// Timeout (in seconds) used when communicating with clients and other servers
    pub timeout: Duration,
}
