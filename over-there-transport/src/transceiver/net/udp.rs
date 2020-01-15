use crate::net::udp;
use crate::transceiver::{
    receiver::{self, ReceiverError},
    transmitter::{self, TransmitterError},
    Context,
};
use over_there_auth::{Signer, Verifier};
use over_there_crypto::{Decrypter, Encrypter};
use std::net::{SocketAddr, UdpSocket};

pub struct UdpTransceiver<A, B>
where
    A: Signer + Verifier,
    B: Encrypter + Decrypter,
{
    pub socket: UdpSocket,
    ctx: Context<A, B>,
}

impl<A, B> UdpTransceiver<A, B>
where
    A: Signer + Verifier,
    B: Encrypter + Decrypter,
{
    pub fn new(socket: UdpSocket, authenticator: A, bicrypter: B) -> Self {
        Self {
            socket,
            ctx: Context::new(udp::MAX_IPV4_DATAGRAM_SIZE, authenticator, bicrypter),
        }
    }

    pub fn send(&mut self, addr: SocketAddr, data: &[u8]) -> Result<(), TransmitterError> {
        let socket = &self.socket;
        transmitter::do_send(From::from(&mut self.ctx), data, |data| {
            // TODO: Support sending remaining bytes in loop? Would need to
            //       support collecting bytes for a packet in multiple receives,
            //       which means we'd need a start and stop indicator of some
            //       kind that is a single byte. Seems too complicated, so
            //       easier to fail and give a reason if we don't send all
            //       of the bytes in one go. It's one of the reasons we made
            //       packets of a guaranteed max size.
            let size = socket.send_to(&data, addr)?;
            if size < data.len() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Only sent {} bytes out of {}", size, data.len()),
                ));
            }

            Ok(size)
        })
    }

    pub fn recv(&mut self) -> Result<(Option<Vec<u8>>, SocketAddr), ReceiverError> {
        let socket = &self.socket;
        receiver::do_receive(From::from(&mut self.ctx), |data| socket.recv_from(data))
    }
}
