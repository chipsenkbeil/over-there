use super::Msg;
use super::{MsgReceiver, MsgReceiverError};
use over_there_auth::Verifier;
use over_there_crypto::Decrypter;
use std::io::Read;
use std::net::TcpStream;

pub struct TcpMsgReceiver<'a, V, D>
where
    V: Verifier,
    D: Decrypter,
{
    pub stream: &'a mut TcpStream,
    msg_receiver: &'a MsgReceiver<'a, V, D>,
}

impl<'a, V, D> TcpMsgReceiver<'a, V, D>
where
    V: Verifier,
    D: Decrypter,
{
    pub fn new(stream: &'a mut TcpStream, msg_receiver: &'a MsgReceiver<'a, V, D>) -> Self {
        Self {
            stream,
            msg_receiver,
        }
    }

    /// Receives data from the underlying stream, yielding a message if
    /// the final packet has been received
    pub fn recv(&mut self) -> Result<Option<Msg>, MsgReceiverError> {
        let s = &mut self.stream;
        self.msg_receiver.recv(|buf| s.read(buf))
    }
}
