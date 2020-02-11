pub mod client;
mod parsers;
pub mod server;

use clap::Clap;

#[derive(Clap, Debug)]
pub enum Command {
    #[clap(name = "client")]
    Client(client::ClientCommand),

    #[clap(name = "server")]
    Server(server::ServerCommand),
}

#[derive(Clap, Debug)]
#[clap(author, about, version)]
pub struct Opts {
    #[clap(subcommand)]
    pub command: Command,
}
