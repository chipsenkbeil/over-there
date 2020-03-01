mod client;
mod event;
mod msg;
mod server;

pub use client::{
    error::AskError,
    error::ExecAskError,
    error::FileAskError,
    error::TellError,
    file::RemoteFile,
    proc::{RemoteProc, RemoteProcStatus},
    Client,
};
pub use event::{AddrEventManager, EventManager};
pub use msg::{
    content::{self, Content},
    Msg, MsgError,
};
pub use server::{
    dir::{LocalDir, LocalDirEntriesError, LocalDirEntry},
    file::{
        LocalFile, LocalFileReadError, LocalFileReadIoError,
        LocalFileWriteError, LocalFileWriteIoError,
    },
    proc::{ExitStatus, LocalProc},
    Server,
};

use over_there_wire::{Authenticator, Bicrypter};
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
pub struct Communicator<A, B>
where
    A: Authenticator,
    B: Bicrypter,
{
    /// TTL to collect all packets for a msg
    packet_ttl: Duration,

    /// Used to sign & verify msgs
    authenticator: A,

    /// Used to encrypt & decrypt msgs
    bicrypter: B,
}

impl<A, B> Communicator<A, B>
where
    A: Authenticator,
    B: Bicrypter,
{
    pub fn new(packet_ttl: Duration, authenticator: A, bicrypter: B) -> Self {
        Self {
            packet_ttl,
            authenticator,
            bicrypter,
        }
    }
}
