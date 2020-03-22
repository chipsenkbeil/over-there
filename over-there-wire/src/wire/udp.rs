use super::{
    auth, crypto, Authenticator, Bicrypter, Decrypter, Encrypter, InboundWire,
    InboundWireError, OutboundWire, OutboundWireError, Signer, Verifier, Wire,
};
use std::net::SocketAddr;
use tokio::net::{
    udp::{RecvHalf, SendHalf},
    UdpSocket,
};

pub struct UdpSocketWire<A, B>
where
    A: Authenticator,
    B: Bicrypter,
{
    wire: Wire<A, B>,
    socket: UdpSocket,
}

impl<A, B> UdpSocketWire<A, B>
where
    A: Authenticator,
    B: Bicrypter,
{
    pub fn new(wire: Wire<A, B>, socket: UdpSocket) -> Self {
        Self { wire, socket }
    }

    pub fn arc_split(
        self,
    ) -> (
        UdpSocketInboundWire<
            auth::split::VerifierHalf<A>,
            crypto::split::DecrypterHalf<B>,
        >,
        UdpSocketOutboundWire<
            auth::split::SignerHalf<A>,
            crypto::split::EncrypterHalf<B>,
        >,
    ) {
        let Self { wire, socket } = self;
        let (r, s) = socket.split();
        let (iw, ow) = wire.arc_split();

        (iw.with_udp_socket(r), ow.with_udp_socket(s))
    }
}

impl<A, B> UdpSocketWire<A, B>
where
    A: Authenticator + Clone,
    B: Bicrypter + Clone,
{
    pub fn clone_split(
        self,
    ) -> (UdpSocketInboundWire<A, B>, UdpSocketOutboundWire<A, B>) {
        let Self { wire, socket } = self;
        let (r, s) = socket.split();
        let (iw, ow) = wire.clone_split();
        (iw.with_udp_socket(r), ow.with_udp_socket(s))
    }
}

pub struct UdpSocketInboundWire<V, D>
where
    V: Verifier,
    D: Decrypter,
{
    inbound_wire: InboundWire<V, D>,
    socket: RecvHalf,
}

impl<V, D> UdpSocketInboundWire<V, D>
where
    V: Verifier,
    D: Decrypter,
{
    pub fn new(inbound_wire: InboundWire<V, D>, socket: RecvHalf) -> Self {
        Self {
            inbound_wire,
            socket,
        }
    }

    pub async fn read(
        &mut self,
    ) -> Result<(Option<Vec<u8>>, SocketAddr), InboundWireError> {
        let mut buf =
            vec![0; self.inbound_wire.transmission_size()].into_boxed_slice();
        let (size, addr) = self
            .socket
            .recv_from(&mut buf)
            .await
            .map_err(InboundWireError::IO)?;
        let data = self.inbound_wire.process(&buf[..size])?;

        Ok((data, addr))
    }
}

pub struct UdpSocketOutboundWire<S, E>
where
    S: Signer,
    E: Encrypter,
{
    outbound_wire: OutboundWire<S, E>,
    socket: SendHalf,
}

impl<S, E> UdpSocketOutboundWire<S, E>
where
    S: Signer,
    E: Encrypter,
{
    pub fn new(outbound_wire: OutboundWire<S, E>, socket: SendHalf) -> Self {
        Self {
            outbound_wire,
            socket,
        }
    }

    pub async fn write_to(
        &mut self,
        buf: &[u8],
        addr: SocketAddr,
    ) -> Result<(), OutboundWireError> {
        let data = self.outbound_wire.process(buf)?;

        for packet_bytes in data.iter() {
            let size = self
                .socket
                .send_to(packet_bytes, &addr)
                .await
                .map_err(OutboundWireError::IO)?;
            if size < packet_bytes.len() {
                return Err(OutboundWireError::IncompleteSend);
            }
        }

        Ok(())
    }
}
