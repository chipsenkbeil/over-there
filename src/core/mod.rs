mod client;
mod event;
mod msg;
mod server;

pub use crate::transport::net;
pub use client::{
    error::AskError,
    error::ExecAskError,
    error::FileAskError,
    error::SendError,
    file::RemoteFile,
    proc::{RemoteProc, RemoteProcStatus},
    Client, ClientBuilder, ConnectedClient,
};
pub use event::{AddrEventManager, EventManager};
pub use msg::{
    content::{
        reply, reply::Capability, request, Content, LazilyTransformedRequest,
        Reply, ReplyError, Request, TransformRequestError, TransformRule,
    },
    Header, Msg, MsgError,
};
pub use server::{
    fs::{FileSystemManager, LocalDirEntry, LocalFile, LocalFileHandle},
    proc::{ExitStatus, LocalProc},
    ListeningServer, Server, ServerBuilder,
};

use std::net::SocketAddr;

/// Transportation medium to use with the client/server
#[derive(Clone, Debug)]
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

pub trait SchemaInfo: schemars::JsonSchema {
    /// Outputs schema as a pretty JSON string
    fn schema() -> String {
        let schema = schemars::schema_for!(Self);
        serde_json::to_string_pretty(&schema)
            .expect("Failed to serialize schema")
    }
}
