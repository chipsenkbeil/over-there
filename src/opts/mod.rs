pub mod client;
mod parsers;
pub mod server;
pub mod types;

use clap::Clap;
use std::time::Duration;
use strum::VariantNames;

#[derive(Clap, Debug)]
pub enum Command {
    #[clap(name = "client")]
    Client(client::ClientCommand),

    #[clap(name = "server")]
    Server(server::ServerCommand),
}

impl Command {
    pub fn common_opts(&self) -> &CommonOpts {
        match self {
            Self::Client(c) => &c.opts,
            Self::Server(s) => &s.opts,
        }
    }
}

#[derive(Clap, Debug)]
#[clap(author, about, version)]
pub struct Opts {
    #[clap(subcommand)]
    pub command: Command,
}

#[derive(Clap, Debug)]
pub struct CommonOpts {
    #[clap(long, parse(try_from_str = parsers::parse_duration), default_value = "5")]
    /// Timeout (in seconds) used when communicating across the network
    pub timeout: Duration,

    #[clap(long, parse(try_from_str = parsers::parse_duration), default_value = "300")]
    /// Time-to-live (in seconds) for collecting all packets in a msg
    pub packet_ttl: Duration,

    #[clap(long, default_value = "1000")]
    /// Maximum size of internal message passing between reader, writer, and
    /// executor loops
    pub internal_buffer_size: usize,

    #[clap(
        short = "t", 
        long, 
        parse(try_from_str), 
        possible_values = &types::Transport::VARIANTS, 
        default_value = "Udp"
    )]
    pub transport: types::Transport,

    #[clap(
        short = "e", 
        long, 
        parse(try_from_str), 
        possible_values = &types::Encryption::VARIANTS, 
        default_value = "None"
    )]
    /// Type of encryption to use with incoming and outgoing msgs
    pub encryption: types::Encryption,

    #[clap(long = "ekey")]
    /// Key to use with encryption
    pub encryption_key: Option<String>,

    #[clap(
        short = "a",
        long,
        parse(try_from_str),
        possible_values = &types::Authentication::VARIANTS,
        default_value = "None"
    )]
    /// Type of authentication to use with incoming and outgoing msgs
    pub authentication: types::Authentication,

    #[clap(long = "akey")]
    /// Key to use with encryption
    pub authentication_key: Option<String>,
}
