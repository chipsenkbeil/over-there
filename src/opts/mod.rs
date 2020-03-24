pub mod client;
mod parsers;
pub mod server;
pub mod types;

use clap::Clap;
use std::time::Duration;
use strum::VariantNames;

#[derive(Clap, Debug)]
pub enum Command {
    /// Launches a client to talk to a server
    #[clap(name = "client")]
    Client(client::ClientCommand),

    /// Launches a server to listen for incoming requests
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
    /// Timeout (in seconds) used when communicating across the network
    #[clap(long, parse(try_from_str = parsers::parse_duration_secs), default_value = "5")]
    pub timeout: Duration,

    /// Time-to-live (in seconds) for collecting all packets in a msg
    #[clap(long, parse(try_from_str = parsers::parse_duration_secs), default_value = "300")]
    pub packet_ttl: Duration,

    /// Maximum size of internal message passing between reader, writer, and
    /// executor loops
    #[clap(long, default_value = "1000")]
    pub internal_buffer_size: usize,

    /// Transportation medium used in communication between client and server
    #[clap(
        short = "t", 
        long, 
        parse(try_from_str), 
        possible_values = &types::Transport::VARIANTS, 
        default_value = "Udp"
    )]
    pub transport: types::Transport,

    /// Type of encryption to use with incoming and outgoing msgs
    #[clap(
        short = "e", 
        long, 
        parse(try_from_str), 
        possible_values = &types::Encryption::VARIANTS, 
        default_value = "None"
    )]
    pub encryption: types::Encryption,

    /// Key to use with encryption
    #[clap(long = "ekey")]
    pub encryption_key: Option<String>,

    /// Type of authentication to use with incoming and outgoing msgs
    #[clap(
        short = "a",
        long,
        parse(try_from_str),
        possible_values = &types::Authentication::VARIANTS,
        default_value = "None"
    )]
    pub authentication: types::Authentication,

    /// Key to use with encryption
    #[clap(long = "akey")]
    pub authentication_key: Option<String>,
}
