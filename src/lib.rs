pub mod client;
pub mod server;

use clap::Clap;
use over_there_core::Client;

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

pub fn hello() {
    println!("HELLO! {:?}", Client::DEFAULT_TIMEOUT);
}
