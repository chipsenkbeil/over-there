use crate::msg::Msg;
use crate::transmitter::data::DataTransmitter;
use crate::transmitter::msg::{Error, MsgTransmitter};
use over_there_transport::tcp;
use std::io::{Read, Write};
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
        let data_transmitter = DataTransmitter::new(tcp::MTU_ETHERNET as u32);
        let msg_transmitter = MsgTransmitter::new(data_transmitter);
        Self::new(stream, msg_transmitter)
    }

    /// Sends a message using the underlying stream
    pub fn send(&mut self, msg: Msg) -> Result<(), Error> {
        let mut s = &self.stream;
        self.msg_transmitter.send(msg, |data| s.write_all(&data))
    }

    /// Receives data from the underlying stream, yielding a message if
    /// the final packet has been received
    pub fn recv(&mut self) -> Result<Option<Msg>, Error> {
        let mut s = &self.stream;
        self.msg_transmitter.recv(|buf| s.read(buf))
    }
}
