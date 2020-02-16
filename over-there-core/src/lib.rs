mod client;
mod event;
mod msg;
mod server;

pub use client::{
    error::AskError, error::ExecAskError, error::FileAskError, error::TellError, file::RemoteFile,
    proc::RemoteProc, Client,
};
pub use event::{AddrEventManager, EventManager};
pub use msg::{
    content::{self, Content},
    Msg, MsgError,
};
pub use server::{file::LocalFile, proc::LocalProc, Server};

use over_there_wire::{Decrypter, Encrypter, Signer, Verifier};
use std::net::SocketAddr;
use std::time::Duration;

/// Transportation medium to use with the client/server
pub enum Transport {
    /// TCP-based communication
    /// - If binding, will use addr available
    /// - If connecting, will use first addr that succeeds
    Tcp(Vec<SocketAddr>),

    /// UDP-based communication
    /// - If binding, will use addr available
    /// - If connecting, will use first addr that succeeds, which should be
    ///   the very first addr in most cases as no network validation is used
    Udp(Vec<SocketAddr>),
}

/// Represents an generic communicator that can become a client or server
pub struct Communicator<S, V, E, D>
where
    S: Signer,
    V: Verifier,
    E: Encrypter,
    D: Decrypter,
{
    /// TTL to collect all packets for a msg
    packet_ttl: Duration,

    /// Used to sign outbound msgs
    signer: S,

    /// Used to verify inbound msgs
    verifier: V,

    /// Used to encrypt outbound msgs
    encrypter: E,

    /// Used to decrypt inbound msgs
    decrypter: D,
}

impl<S, V, E, D> Communicator<S, V, E, D>
where
    S: Signer + Send + 'static,
    V: Verifier + Send + 'static,
    E: Encrypter + Send + 'static,
    D: Decrypter + Send + 'static,
{
    pub fn new(packet_ttl: Duration, signer: S, verifier: V, encrypter: E, decrypter: D) -> Self {
        Self {
            packet_ttl,
            signer,
            verifier,
            encrypter,
            decrypter,
        }
    }
}
