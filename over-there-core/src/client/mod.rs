mod connect;
pub mod route;
pub mod state;

use crate::{
    msg::{callback::Callback, content::Content, Msg},
    state::State,
};
use log::trace;
use over_there_auth::{Signer, Verifier};
use over_there_crypto::{Decrypter, Encrypter};
use over_there_derive::Error;
use over_there_transport::{
    net, TcpStreamTransceiverError, TransceiverThread, UdpStreamTransceiverError,
};
use std::io;
use std::net::{SocketAddr, TcpStream, UdpSocket};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

#[derive(Debug, Error)]
pub enum ClientError {
    EncodingFailed,
    SendFailed,
}

#[derive(Debug, Error)]
pub enum AskError {
    Failure { msg: String },
    InvalidResponse,
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
    pub fn connect_tcp<A, B, C>(
        remote_addr: SocketAddr,
        packet_ttl: Duration,
        authenticator: A,
        bicrypter: B,
        err_callback: C,
    ) -> Result<Self, io::Error>
    where
        A: Signer + Verifier + Send + Sync + 'static,
        B: Encrypter + Decrypter + Send + Sync + 'static,
        C: Fn(TcpStreamTransceiverError) -> bool + Send + 'static,
    {
        Self::connect_using_tcp_stream(
            TcpStream::connect(remote_addr)?,
            packet_ttl,
            authenticator,
            bicrypter,
            err_callback,
        )
    }

    pub fn connect_using_tcp_stream<A, B, C>(
        stream: TcpStream,
        packet_ttl: Duration,
        authenticator: A,
        bicrypter: B,
        err_callback: C,
    ) -> Result<Self, io::Error>
    where
        A: Signer + Verifier + Send + Sync + 'static,
        B: Encrypter + Decrypter + Send + Sync + 'static,
        C: Fn(TcpStreamTransceiverError) -> bool + Send + 'static,
    {
        connect::tcp_connect(stream, packet_ttl, authenticator, bicrypter, err_callback)
    }

    pub fn connect_udp<A, B, C>(
        remote_addr: SocketAddr,
        packet_ttl: Duration,
        authenticator: A,
        bicrypter: B,
        err_callback: C,
    ) -> Result<Self, io::Error>
    where
        A: Signer + Verifier + Send + Sync + 'static,
        B: Encrypter + Decrypter + Send + Sync + 'static,
        C: Fn(UdpStreamTransceiverError) -> bool + Send + 'static,
    {
        Self::connect_using_udp_socket(
            net::udp::connect(remote_addr)?,
            remote_addr,
            packet_ttl,
            authenticator,
            bicrypter,
            err_callback,
        )
    }

    pub fn connect_using_udp_socket<A, B, C>(
        socket: UdpSocket,
        remote_addr: SocketAddr,
        packet_ttl: Duration,
        authenticator: A,
        bicrypter: B,
        err_callback: C,
    ) -> Result<Self, io::Error>
    where
        A: Signer + Verifier + Send + Sync + 'static,
        B: Encrypter + Decrypter + Send + Sync + 'static,
        C: Fn(UdpStreamTransceiverError) -> bool + Send + 'static,
    {
        connect::udp_connect(
            socket,
            remote_addr,
            packet_ttl,
            authenticator,
            bicrypter,
            err_callback,
        )
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
    pub fn ask(
        &self,
        msg: Msg,
        f: impl FnOnce(Result<&Msg, AskError>) + Send + 'static,
    ) -> Result<(), ClientError> {
        self.state
            .lock()
            .unwrap()
            .callback_manager()
            .add_callback(msg.header.id, move |msg| {
                if let Content::Error { msg } = &msg.content {
                    f(Err(AskError::Failure {
                        msg: msg.to_string(),
                    }));
                } else {
                    f(Ok(msg));
                }
            });

        self.tell(msg)
    }

    /// Sends a msg to the server
    pub fn tell(&self, msg: Msg) -> Result<(), ClientError> {
        trace!("Sending to {}: {:?}", self.remote_addr, msg);
        self.transceiver_thread
            .send(msg.to_vec().map_err(|_| ClientError::EncodingFailed)?)
            .map_err(|_| ClientError::SendFailed)
    }

    /// Requests the version from the server
    pub fn ask_version(
        &self,
        f: impl FnOnce(Result<&String, AskError>) + Send + 'static,
    ) -> Result<(), ClientError> {
        self.ask(Msg::from(Content::VersionRequest), move |result| {
            f(match result.map(|m| &m.content) {
                Ok(Content::VersionResponse { version }) => Ok(&version),
                Ok(_) => Err(AskError::InvalidResponse),
                Err(x) => Err(x),
            })
        })
    }

    /// Requests the version from the server
    pub fn ask_capabilities(
        &self,
        f: impl FnOnce(Result<&Vec<String>, AskError>) + Send + 'static,
    ) -> Result<(), ClientError> {
        self.ask(Msg::from(Content::CapabilitiesRequest), move |result| {
            f(match result.map(|m| &m.content) {
                Ok(Content::CapabilitiesResponse { capabilities }) => Ok(&capabilities),
                Ok(_) => Err(AskError::InvalidResponse),
                Err(x) => Err(x),
            })
        })
    }
}
