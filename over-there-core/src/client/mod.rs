mod connect;
pub mod future;
pub mod state;

use crate::msg::{content::Content, Msg};
use future::{AskFuture, AskFutureState};
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
pub enum TellError {
    EncodingFailed,
    SendFailed,
}

#[derive(Debug, Error)]
pub enum AskError {
    Failure { msg: String },
    InvalidResponse,
    Timeout,
    EncodingFailed,
    SendFailed,
}

pub struct Client {
    state: Arc<Mutex<state::ClientState>>,

    /// Represents the address the client is connected to
    pub remote_addr: SocketAddr,

    /// Represents maximum to wait on responses before timing out
    pub timeout: Duration,

    /// Performs sending/receiving over network
    transceiver_thread: TransceiverThread<Vec<u8>, ()>,

    /// Processes incoming msg structs
    msg_thread: JoinHandle<()>,
}

impl Client {
    pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);

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

    pub fn join(self) -> Result<(), Box<dyn std::error::Error>> {
        self.transceiver_thread.join()?;
        self.msg_thread
            .join()
            .map_err(|_| "Msg Process Thread Join Error")?;

        Ok(())
    }

    /// Generic ask of the server that is expecting a response
    pub async fn ask(&self, msg: Msg) -> Result<Msg, AskError> {
        let state = Arc::new(Mutex::new(AskFutureState::new(self.timeout)));

        let callback_state = Arc::clone(&state);
        self.state
            .lock()
            .unwrap()
            .callback_manager
            .add_callback(msg.header.id, move |msg| {
                let mut s = callback_state.lock().unwrap();
                if let Content::Error { msg } = &msg.content {
                    s.result = Some(Err(AskError::Failure {
                        msg: msg.to_string(),
                    }));
                } else {
                    s.result = Some(Ok(msg.clone()));
                }

                if let Some(waker) = s.waker.take() {
                    waker.wake();
                }
            });

        let f = AskFuture { state };

        // Convert the tell errors to ask equivalents
        match self.tell(msg).await {
            Err(TellError::EncodingFailed) => return Err(AskError::EncodingFailed),
            Err(TellError::SendFailed) => return Err(AskError::SendFailed),
            Ok(_) => (),
        }

        let msg = f.await?;
        Ok(msg)
    }

    /// Sends a msg to the server, not expecting a response
    pub async fn tell(&self, msg: Msg) -> Result<(), TellError> {
        trace!("Sending to {}: {:?}", self.remote_addr, msg);

        // TODO: Make non-blocking, would involve re-writing transport to use
        //       async implementation
        self.transceiver_thread
            .send(msg.to_vec().map_err(|_| TellError::EncodingFailed)?)
            .map_err(|_| TellError::SendFailed)
    }

    /// Requests the version from the server
    pub async fn ask_version(&self) -> Result<String, AskError> {
        let msg = self.ask(Msg::from(Content::VersionRequest)).await?;
        match msg.content {
            Content::VersionResponse { version } => Ok(version),
            _ => Err(AskError::InvalidResponse),
        }
    }

    /// Requests the capabilities from the server
    pub async fn ask_capabilities(&self) -> Result<Vec<String>, AskError> {
        let msg = self.ask(Msg::from(Content::CapabilitiesRequest)).await?;
        match msg.content {
            Content::CapabilitiesResponse { capabilities } => Ok(capabilities),
            _ => Err(AskError::InvalidResponse),
        }
    }
}
