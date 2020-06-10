use super::{
    auth, crypto, Authenticator, Bicrypter, Decrypter, Encrypter, InboundWire,
    InboundWireError, OutboundWire, OutboundWireError, Signer, Verifier, Wire,
};
use std::net::SocketAddr;
use tokio::{
    io::{self, AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf},
    net::TcpStream,
};

pub struct TcpStreamWire<A, B>
where
    A: Authenticator,
    B: Bicrypter,
{
    wire: Wire<A, B>,
    stream: TcpStream,
    remote_addr: SocketAddr,
}

impl<A, B> TcpStreamWire<A, B>
where
    A: Authenticator,
    B: Bicrypter,
{
    pub fn new(
        wire: Wire<A, B>,
        stream: TcpStream,
        remote_addr: SocketAddr,
    ) -> Self {
        Self {
            wire,
            stream,
            remote_addr,
        }
    }

    pub fn arc_split(
        self,
    ) -> (
        TcpStreamInboundWire<
            auth::split::VerifierHalf<A>,
            crypto::split::DecrypterHalf<B>,
        >,
        TcpStreamOutboundWire<
            auth::split::SignerHalf<A>,
            crypto::split::EncrypterHalf<B>,
        >,
    ) {
        let Self {
            wire,
            stream,
            remote_addr,
        } = self;
        let (r, w) = io::split(stream);
        let (iw, ow) = wire.arc_split();

        (iw.with_tcp_stream(r, remote_addr), ow.with_tcp_stream(w))
    }
}

impl<A, B> TcpStreamWire<A, B>
where
    A: Authenticator + Clone,
    B: Bicrypter + Clone,
{
    pub fn clone_split(
        self,
    ) -> (TcpStreamInboundWire<A, B>, TcpStreamOutboundWire<A, B>) {
        let Self {
            wire,
            stream,
            remote_addr,
        } = self;
        let (r, w) = io::split(stream);
        let (iw, ow) = wire.clone_split();
        (iw.with_tcp_stream(r, remote_addr), ow.with_tcp_stream(w))
    }
}

pub struct TcpStreamInboundWire<V, D>
where
    V: Verifier,
    D: Decrypter,
{
    inbound_wire: InboundWire<V, D>,
    stream: ReadHalf<TcpStream>,
    remote_addr: SocketAddr,
}

impl<V, D> TcpStreamInboundWire<V, D>
where
    V: Verifier,
    D: Decrypter,
{
    pub fn new(
        inbound_wire: InboundWire<V, D>,
        stream: ReadHalf<TcpStream>,
        remote_addr: SocketAddr,
    ) -> Self {
        Self {
            inbound_wire,
            stream,
            remote_addr,
        }
    }

    pub async fn read(
        &mut self,
    ) -> Result<(Option<Vec<u8>>, SocketAddr), InboundWireError> {
        let mut buf =
            vec![0; self.inbound_wire.transmission_size()].into_boxed_slice();
        let size = self
            .stream
            .read(&mut buf)
            .await
            .map_err(InboundWireError::IO)?;
        let data = self.inbound_wire.process(&buf[..size])?;

        Ok((data, self.remote_addr))
    }
}

pub struct TcpStreamOutboundWire<S, E>
where
    S: Signer,
    E: Encrypter,
{
    outbound_wire: OutboundWire<S, E>,
    stream: WriteHalf<TcpStream>,
}

impl<S, E> TcpStreamOutboundWire<S, E>
where
    S: Signer,
    E: Encrypter,
{
    pub fn new(
        outbound_wire: OutboundWire<S, E>,
        stream: WriteHalf<TcpStream>,
    ) -> Self {
        Self {
            outbound_wire,
            stream,
        }
    }

    pub async fn write(&mut self, buf: &[u8]) -> Result<(), OutboundWireError> {
        let data = self.outbound_wire.process(buf)?;

        for packet_bytes in data.iter() {
            let size = self
                .stream
                .write(packet_bytes)
                .await
                .map_err(OutboundWireError::IO)?;
            if size < packet_bytes.len() {
                return Err(OutboundWireError::IncompleteSend);
            }
        }

        Ok(())
    }
}
