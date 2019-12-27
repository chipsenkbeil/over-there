use super::Msg;
use super::{Error, MsgTransmitter};
use over_there_transport::udp;
use over_there_transport::Transmitter;
use std::io;
use std::net::{SocketAddr, UdpSocket};

pub struct UdpMsgTransmitter {
    pub socket: UdpSocket,
    msg_transmitter: MsgTransmitter,
}

impl UdpMsgTransmitter {
    pub fn new(socket: UdpSocket, msg_transmitter: MsgTransmitter) -> Self {
        UdpMsgTransmitter {
            socket,
            msg_transmitter,
        }
    }

    pub fn from_socket(socket: UdpSocket) -> Self {
        let transmitter = Transmitter::new(udp::MAX_IPV4_DATAGRAM_SIZE as u32);
        let msg_transmitter = MsgTransmitter::new(transmitter);
        Self::new(socket, msg_transmitter)
    }

    /// Sends a message to the specified address using the underlying socket
    pub fn send(&self, msg: Msg, addr: SocketAddr) -> Result<(), Error> {
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

    /// Receives data from the underlying socket, yielding a message and
    /// origin address if the final packet has been received
    pub fn recv(&self) -> Result<Option<(Msg, SocketAddr)>, Error> {
        let mut addr: Option<SocketAddr> = None;
        let msg = self.msg_transmitter.recv(|buf| {
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
