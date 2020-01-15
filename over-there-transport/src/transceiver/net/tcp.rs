use crate::net::tcp;
use crate::transceiver::{
    receiver::{self, ReceiverError},
    transmitter::{self, TransmitterError},
    Context,
};
use over_there_auth::{Signer, Verifier};
use over_there_crypto::{Decrypter, Encrypter};
use std::io::{Read, Write};
use std::net::TcpStream;

pub struct TcpStreamTransceiver<A, B>
where
    A: Signer + Verifier,
    B: Encrypter + Decrypter,
{
    pub stream: TcpStream,
    ctx: Context<A, B>,
}

impl<A, B> TcpStreamTransceiver<A, B>
where
    A: Signer + Verifier,
    B: Encrypter + Decrypter,
{
    pub fn new(stream: TcpStream, authenticator: A, bicrypter: B) -> Self {
        Self {
            stream,
            ctx: Context::new(tcp::MTU_ETHERNET_SIZE, authenticator, bicrypter),
        }
    }

    pub fn send(&mut self, data: &[u8]) -> Result<(), TransmitterError> {
        let stream = &mut self.stream;
        transmitter::do_send(From::from(&mut self.ctx), data, |data| {
            // TODO: Support sending remaining bytes in loop? Would need to
            //       support collecting bytes for a packet in multiple receives,
            //       which means we'd need a start and stop indicator of some
            //       kind that is a single byte. Seems too complicated, so
            //       easier to fail and give a reason if we don't send all
            //       of the bytes in one go. It's one of the reasons we made
            //       packets of a guaranteed max size.
            let size = stream.write(&data)?;
            if size < data.len() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Only sent {} bytes out of {}", size, data.len()),
                ));
            }

            Ok(size)
        })
    }

    pub fn recv(&mut self) -> Result<Option<Vec<u8>>, ReceiverError> {
        let stream = &mut self.stream;
        receiver::do_receive(From::from(&mut self.ctx), |data| {
            stream.read(data).map(|s| (s, ()))
        })
        .map(|r| r.0)
    }
}
