use super::Msg;
use super::{Error, MsgTransmitter};
use over_there_transport::tcp;
use over_there_transport::Transmitter;
use std::io::{self, Read, Write};
use std::net::TcpStream;

pub struct TcpMsgTransmitter {
    pub stream: TcpStream,
    msg_transmitter: MsgTransmitter,
}

impl TcpMsgTransmitter {
    pub fn new(stream: TcpStream, msg_transmitter: MsgTransmitter) -> Self {
        TcpMsgTransmitter {
            stream,
            msg_transmitter,
        }
    }

    pub fn from_stream(stream: TcpStream) -> Self {
        let transmitter = Transmitter::new(tcp::MTU_ETHERNET_SIZE as u32);
        let msg_transmitter = MsgTransmitter::new(transmitter);
        Self::new(stream, msg_transmitter)
    }

    /// Sends a message using the underlying stream
    pub fn send(&mut self, msg: Msg) -> Result<(), Error> {
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
    pub fn recv(&mut self) -> Result<Option<Msg>, Error> {
        let mut s = &self.stream;
        self.msg_transmitter.recv(|buf| s.read(buf))
    }
}