use super::Msg;
use super::{MsgTransmitter, MsgTransmitterError};
use over_there_crypto::Bicrypter;
use over_there_transport::tcp;
use over_there_transport::Transmitter;
use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::time::Duration;

pub struct TcpMsgTransmitter<B>
where
    B: Bicrypter,
{
    pub stream: TcpStream,
    msg_transmitter: MsgTransmitter<B>,
}

impl<B> TcpMsgTransmitter<B>
where
    B: Bicrypter,
{
    pub fn new(stream: TcpStream, msg_transmitter: MsgTransmitter<B>) -> Self {
        Self {
            stream,
            msg_transmitter,
        }
    }

    pub fn from_stream(
        stream: TcpStream,
        cache_capacity: usize,
        cache_duration: Duration,
        bicrypter: B,
    ) -> Self {
        let transmitter = Transmitter::new(
            tcp::MTU_ETHERNET_SIZE,
            cache_capacity,
            cache_duration,
            bicrypter,
        );
        let msg_transmitter = MsgTransmitter::new(transmitter);
        Self::new(stream, msg_transmitter)
    }

    /// Sends a message using the underlying stream
    pub fn send(&mut self, msg: Msg) -> Result<(), MsgTransmitterError> {
        let mut s = &self.stream;
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

    /// Receives data from the underlying stream, yielding a message if
    /// the final packet has been received
    pub fn recv(&mut self) -> Result<Option<Msg>, MsgTransmitterError> {
        let mut s = &self.stream;
        self.msg_transmitter.recv(|buf| s.read(buf))
    }
}
