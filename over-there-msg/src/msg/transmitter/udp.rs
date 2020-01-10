use super::Msg;
use super::{MsgTransmitter, MsgTransmitterError};
use over_there_auth::Signer;
use over_there_crypto::Encrypter;
use std::io;
use std::net::{SocketAddr, UdpSocket};

pub struct UdpMsgTransmitter<'a, S, E>
where
    S: Signer,
    E: Encrypter,
{
    pub socket: &'a UdpSocket,
    msg_transmitter: &'a MsgTransmitter<'a, S, E>,
}

impl<'a, S, E> UdpMsgTransmitter<'a, S, E>
where
    S: Signer,
    E: Encrypter,
{
    pub fn new(socket: &'a UdpSocket, msg_transmitter: &'a MsgTransmitter<'a, S, E>) -> Self {
        Self {
            socket,
            msg_transmitter,
        }
    }

    /// Sends a message to the specified address using the underlying socket
    pub fn send(&self, msg: Msg, addr: SocketAddr) -> Result<(), MsgTransmitterError> {
        self.msg_transmitter.send(msg, |data| {
            // TODO: Support sending remaining bytes in loop? Would need to
            //       support collecting bytes for a packet in multiple receives,
            //       which means we'd need a start and stop indicator of some
            //       kind that is a single byte. Seems too complicated, so
            //       easier to fail and give a reason if we don't send all
            //       of the bytes in one go. It's one of the reasons we made
            //       packets of a guaranteed max size.
            let size = self.socket.send_to(&data, addr)?;
            if size < data.len() {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    format!("Only sent {} bytes out of {}", size, data.len()),
                ));
            }

            Ok(())
        })
    }
}
