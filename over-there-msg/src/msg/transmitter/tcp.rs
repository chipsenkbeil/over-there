use super::Msg;
use super::{MsgTransmitter, MsgTransmitterError};
use over_there_auth::Signer;
use over_there_crypto::Encrypter;
use std::io::{self, Write};
use std::net::TcpStream;

pub struct TcpMsgTransmitter<'a, S, E>
where
    S: Signer,
    E: Encrypter,
{
    pub stream: &'a mut TcpStream,
    msg_transmitter: &'a MsgTransmitter<'a, S, E>,
}

impl<'a, S, E> TcpMsgTransmitter<'a, S, E>
where
    S: Signer,
    E: Encrypter,
{
    pub fn new(stream: &'a mut TcpStream, msg_transmitter: &'a MsgTransmitter<'a, S, E>) -> Self {
        Self {
            stream,
            msg_transmitter,
        }
    }

    /// Sends a message using the underlying stream
    pub fn send(&mut self, msg: Msg) -> Result<(), MsgTransmitterError> {
        let s = &mut self.stream;
        self.msg_transmitter.send(msg, |data| {
            // TODO: Support sending remaining bytes in loop? Would need to
            //       support collecting bytes for a packet in multiple receives,
            //       which means we'd need a start and stop indicator of some
            //       kind that is a single byte. Seems too complicated, so
            //       easier to fail and give a reason if we don't send all
            //       of the bytes in one go. It's one of the reasons we made
            //       packets of a guaranteed max size.
            let size = s.write(&data)?;
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
