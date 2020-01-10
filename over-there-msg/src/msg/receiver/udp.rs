use super::Msg;
use super::{MsgReceiver, MsgReceiverError};
use over_there_auth::Verifier;
use over_there_crypto::Decrypter;
use std::net::{SocketAddr, UdpSocket};

pub struct UdpMsgReceiver<'a, V, D>
where
    V: Verifier,
    D: Decrypter,
{
    pub socket: &'a UdpSocket,
    msg_receiver: &'a MsgReceiver<'a, V, D>,
}

impl<'a, V, D> UdpMsgReceiver<'a, V, D>
where
    V: Verifier,
    D: Decrypter,
{
    pub fn new(socket: &'a UdpSocket, msg_receiver: &'a MsgReceiver<'a, V, D>) -> Self {
        Self {
            socket,
            msg_receiver,
        }
    }

    /// Receives data from the underlying socket, yielding a message and
    /// origin address if the final packet has been received
    pub fn recv(&self) -> Result<Option<(Msg, SocketAddr)>, MsgReceiverError> {
        let mut addr: Option<SocketAddr> = None;
        let msg = self.msg_receiver.recv(|buf| {
            let (size, src) = self.socket.recv_from(buf)?;
            addr = Some(src);
            Ok(size)
        })?;
        Ok(match (msg, addr) {
            (Some(m), Some(a)) => Some((m, a)),
            _ => None,
        })
    }
}
