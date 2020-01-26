mod connect;
pub mod route;
pub mod state;

use crate::{
    msg::{callback::Callback, content::Content, Msg},
    state::State,
};
use over_there_auth::{Signer, Verifier};
use over_there_crypto::{Decrypter, Encrypter};
use over_there_derive::Error;
use over_there_transport::{net, TransceiverThread};
use std::io;
use std::net::{SocketAddr, TcpStream, UdpSocket};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;
use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll, Waker},
};

#[derive(Debug, Error)]
pub enum ClientError {
    EncodingFailed,
    SendFailed,
}

pub struct Client {
    state: Arc<Mutex<state::ClientState>>,

    /// Represents the address the client is connected to
    pub remote_addr: SocketAddr,

    /// Performs sending/receiving over network
    transceiver_thread: TransceiverThread<Vec<u8>, ()>,

    /// Processes incoming msg structs
    msg_thread: JoinHandle<()>,
}

impl Client {
    pub fn connect_tcp<A, B>(
        remote_addr: SocketAddr,
        packet_ttl: Duration,
        authenticator: A,
        bicrypter: B,
    ) -> Result<Self, io::Error>
    where
        A: Signer + Verifier + Send + Sync + 'static,
        B: Encrypter + Decrypter + Send + Sync + 'static,
    {
        Self::connect_using_tcp_stream(
            TcpStream::connect(remote_addr)?,
            packet_ttl,
            authenticator,
            bicrypter,
        )
    }

    pub fn connect_using_tcp_stream<A, B>(
        stream: TcpStream,
        packet_ttl: Duration,
        authenticator: A,
        bicrypter: B,
    ) -> Result<Self, io::Error>
    where
        A: Signer + Verifier + Send + Sync + 'static,
        B: Encrypter + Decrypter + Send + Sync + 'static,
    {
        connect::tcp_connect(stream, packet_ttl, authenticator, bicrypter)
    }

    pub fn connect_udp<A, B>(
        remote_addr: SocketAddr,
        packet_ttl: Duration,
        authenticator: A,
        bicrypter: B,
    ) -> Result<Self, io::Error>
    where
        A: Signer + Verifier + Send + Sync + 'static,
        B: Encrypter + Decrypter + Send + Sync + 'static,
    {
        Self::connect_using_udp_socket(
            net::udp::connect(remote_addr)?,
            remote_addr,
            packet_ttl,
            authenticator,
            bicrypter,
        )
    }

    pub fn connect_using_udp_socket<A, B>(
        socket: UdpSocket,
        remote_addr: SocketAddr,
        packet_ttl: Duration,
        authenticator: A,
        bicrypter: B,
    ) -> Result<Self, io::Error>
    where
        A: Signer + Verifier + Send + Sync + 'static,
        B: Encrypter + Decrypter + Send + Sync + 'static,
    {
        connect::udp_connect(socket, remote_addr, packet_ttl, authenticator, bicrypter)
    }

    pub fn add_callback(&mut self, id: u32, callback: impl FnMut(&Msg) + Send + 'static) {
        self.state
            .lock()
            .unwrap()
            .callback_manager()
            .add_callback(id, callback)
    }

    pub fn take_callback(&mut self, id: u32) -> Option<Box<Callback>> {
        self.state
            .lock()
            .unwrap()
            .callback_manager()
            .take_callback(id)
    }

    pub fn join(self) -> Result<(), Box<dyn std::error::Error>> {
        self.transceiver_thread.join()?;
        self.msg_thread
            .join()
            .map_err(|_| "Msg Process Thread Join Error")?;

        Ok(())
    }

    /// Generic ask of the server that is expecting a response, which
    /// will be passed back to the callback
    pub fn ask(&self, msg: Msg, f: impl FnMut(&Msg) + Send + 'static) -> Result<(), ClientError> {
        self.state
            .lock()
            .unwrap()
            .callback_manager()
            .add_callback(msg.header.id, f);

        self.transceiver_thread
            .send(msg.to_vec().map_err(|_| ClientError::EncodingFailed)?)
            .map_err(|_| ClientError::SendFailed)
    }

    /// Requests the version from the server
    pub fn ask_version(
        &self,
        mut f: impl FnMut(String) + Send + 'static,
    ) -> Result<(), ClientError> {
        self.ask(Msg::from(Content::VersionRequest), move |msg| {
            if let Content::VersionResponse { version } = &msg.content {
                f(version.to_string());
            }
        })
    }
}
