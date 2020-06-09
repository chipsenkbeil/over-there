mod input;
mod output;
mod packet;
pub mod tcp;
pub mod udp;

use crate::transport::auth::{self as auth, Authenticator, Signer, Verifier};
use crate::transport::crypto::{
    self as crypto, Bicrypter, Decrypter, Encrypter,
};
use derive_more::{Display, Error};
use std::io;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::{TcpStream, UdpSocket};

// Export errors
pub use input::decoder::DecoderError;
pub use input::{InputProcessor, InputProcessorError};
pub use output::encoder::EncoderError;
pub use output::{OutputProcessor, OutputProcessorError};

#[derive(Debug, Clone)]
pub struct Wire<A, B>
where
    A: Authenticator,
    B: Bicrypter,
{
    transmission_size: usize,
    packet_ttl: Duration,
    authenticator: A,
    bicrypter: B,
}

impl<A, B> Wire<A, B>
where
    A: Authenticator,
    B: Bicrypter,
{
    pub fn new(
        transmission_size: usize,
        packet_ttl: Duration,
        authenticator: A,
        bicrypter: B,
    ) -> Self {
        Self {
            transmission_size,
            packet_ttl,
            authenticator,
            bicrypter,
        }
    }

    pub fn transmission_size(&self) -> usize {
        self.transmission_size
    }

    pub fn packet_ttl(&self) -> Duration {
        self.packet_ttl
    }

    pub fn with_tcp_stream(
        self,
        stream: TcpStream,
        remote_addr: SocketAddr,
    ) -> tcp::TcpStreamWire<A, B> {
        tcp::TcpStreamWire::new(self, stream, remote_addr)
    }

    pub fn with_udp_socket(
        self,
        socket: UdpSocket,
    ) -> udp::UdpSocketWire<A, B> {
        udp::UdpSocketWire::new(self, socket)
    }

    pub fn arc_split(
        self,
    ) -> (
        InboundWire<
            auth::split::VerifierHalf<A>,
            crypto::split::DecrypterHalf<B>,
        >,
        OutboundWire<
            auth::split::SignerHalf<A>,
            crypto::split::EncrypterHalf<B>,
        >,
    ) {
        let Self {
            transmission_size,
            packet_ttl,
            authenticator,
            bicrypter,
        } = self;

        let (signer, verifier) = auth::split::split(authenticator);
        let (encrypter, decrypter) = crypto::split::split(bicrypter);
        new_inbound_outbound_wires(
            transmission_size,
            packet_ttl,
            signer,
            verifier,
            encrypter,
            decrypter,
        )
    }
}

impl<A, B> Wire<A, B>
where
    A: Authenticator + Clone,
    B: Bicrypter + Clone,
{
    pub fn clone_split(self) -> (InboundWire<A, B>, OutboundWire<A, B>) {
        let Self {
            transmission_size,
            packet_ttl,
            authenticator,
            bicrypter,
        } = self;
        let (signer, verifier) = auth::split::clone_split(authenticator);
        let (encrypter, decrypter) = crypto::split::clone_split(bicrypter);
        new_inbound_outbound_wires(
            transmission_size,
            packet_ttl,
            signer,
            verifier,
            encrypter,
            decrypter,
        )
    }
}

#[derive(Debug, Display, Error)]
pub enum InboundWireError {
    IO(io::Error),
    InputProcessor(InputProcessorError),
}

/// Wire for inbound communication
#[derive(Debug, Clone)]
pub struct InboundWire<V, D>
where
    V: Verifier,
    D: Decrypter,
{
    /// Maximum size of expected data
    transmission_size: usize,

    /// Processes input coming into the wire
    input_processor: InputProcessor<V, D>,
}

impl<V, D> InboundWire<V, D>
where
    V: Verifier,
    D: Decrypter,
{
    pub fn new(
        transmission_size: usize,
        packet_ttl: Duration,
        verifier: V,
        decrypter: D,
    ) -> Self {
        let input_processor =
            InputProcessor::new(packet_ttl, verifier, decrypter);
        Self {
            transmission_size,
            input_processor,
        }
    }

    pub fn transmission_size(&self) -> usize {
        self.transmission_size
    }

    pub fn with_tcp_stream(
        self,
        stream: tokio::io::ReadHalf<TcpStream>,
        remote_addr: SocketAddr,
    ) -> tcp::TcpStreamInboundWire<V, D> {
        tcp::TcpStreamInboundWire::new(self, stream, remote_addr)
    }

    pub fn with_udp_socket(
        self,
        socket: tokio::net::udp::RecvHalf,
    ) -> udp::UdpSocketInboundWire<V, D> {
        udp::UdpSocketInboundWire::new(self, socket)
    }

    #[inline]
    pub fn process(
        &mut self,
        buf: &[u8],
    ) -> Result<Option<Vec<u8>>, InboundWireError> {
        self.input_processor
            .process(buf)
            .map_err(InboundWireError::InputProcessor)
    }
}

#[derive(Debug, Display, Error)]
pub enum OutboundWireError {
    IO(io::Error),
    OutputProcessor(output::OutputProcessorError),

    /// When fail to send all bytes out together on the wire
    IncompleteSend,
}

/// Wire for outbound communication
#[derive(Debug, Clone)]
pub struct OutboundWire<S, E>
where
    S: Signer,
    E: Encrypter,
{
    /// Processes output leaving on the wire
    output_processor: OutputProcessor<S, E>,
}

impl<S, E> OutboundWire<S, E>
where
    S: Signer,
    E: Encrypter,
{
    pub fn new(transmission_size: usize, signer: S, encrypter: E) -> Self {
        let output_processor =
            OutputProcessor::new(transmission_size, signer, encrypter);
        Self { output_processor }
    }

    pub fn with_tcp_stream(
        self,
        stream: tokio::io::WriteHalf<TcpStream>,
    ) -> tcp::TcpStreamOutboundWire<S, E> {
        tcp::TcpStreamOutboundWire::new(self, stream)
    }

    pub fn with_udp_socket(
        self,
        socket: tokio::net::udp::SendHalf,
    ) -> udp::UdpSocketOutboundWire<S, E> {
        udp::UdpSocketOutboundWire::new(self, socket)
    }

    #[inline]
    pub fn process(
        &mut self,
        buf: &[u8],
    ) -> Result<Vec<Vec<u8>>, OutboundWireError> {
        self.output_processor
            .process(buf)
            .map_err(OutboundWireError::OutputProcessor)
    }
}

fn new_inbound_outbound_wires<S, V, E, D>(
    transmission_size: usize,
    packet_ttl: Duration,
    signer: S,
    verifier: V,
    encrypter: E,
    decrypter: D,
) -> (InboundWire<V, D>, OutboundWire<S, E>)
where
    S: Signer,
    V: Verifier,
    E: Encrypter,
    D: Decrypter,
{
    let inbound_wire =
        InboundWire::new(transmission_size, packet_ttl, verifier, decrypter);
    let outbound_wire = OutboundWire::new(transmission_size, signer, encrypter);

    (inbound_wire, outbound_wire)
}
