pub mod tcp;
pub mod udp;

use super::{
    receiver::ReceiverError, transmitter::TransmitterError, Responder, ResponderError,
    TransceiverThread,
};
use crate::net;
use std::io;
use std::net::SocketAddr;
use std::sync::mpsc;
use std::time::Duration;

pub type Data = Vec<u8>;
pub type DataAndAddr = (Data, SocketAddr);

pub enum NetTransmission {
    TcpEthernet,
    TcpDialup,
    UdpIpv4,
    UdpIpv6,
}

impl NetTransmission {
    pub fn size(&self) -> usize {
        match self {
            Self::TcpEthernet => net::tcp::MTU_ETHERNET_SIZE,
            Self::TcpDialup => net::tcp::MTU_DIALUP_SIZE,
            Self::UdpIpv4 => net::udp::MAX_IPV4_DATAGRAM_SIZE,
            Self::UdpIpv6 => net::udp::MAX_IPV6_DATAGRAM_SIZE,
        }
    }
}

impl Into<usize> for NetTransmission {
    fn into(self) -> usize {
        self.size()
    }
}

#[derive(Clone, Debug)]
pub struct NetResponder {
    tx: mpsc::Sender<Data>,
}

impl Responder for NetResponder {
    fn send(&self, data: &[u8]) -> Result<(), ResponderError> {
        self.tx
            .send(data.to_vec())
            .map_err(|_| ResponderError::NoLongerAvailable)
    }
}

#[derive(Clone, Debug)]
pub struct AddrNetResponder {
    tx: mpsc::Sender<DataAndAddr>,
    pub addr: SocketAddr,
}

impl Responder for AddrNetResponder {
    fn send(&self, data: &[u8]) -> Result<(), ResponderError> {
        self.tx
            .send((data.to_vec(), self.addr))
            .map_err(|_| ResponderError::NoLongerAvailable)
    }
}

pub trait NetStream {
    type Error: std::error::Error;

    /// Spawns a new transceiver thread to communicate between this stream
    /// and the remote connection
    fn spawn<C, D>(
        self,
        sleep_duration: Duration,
        callback: C,
        err_callback: D,
    ) -> io::Result<TransceiverThread<Data, ()>>
    where
        C: Fn(Vec<u8>, NetResponder) + Send + 'static,
        D: Fn(Self::Error) -> bool + Send + 'static;

    /// Sends data to the remote connection
    fn send(&mut self, data: &[u8]) -> Result<(), TransmitterError>;

    /// Receives data from the remote connection
    fn recv(&mut self) -> Result<Option<Data>, ReceiverError>;
}

pub trait NetListener {
    type Error: std::error::Error;
    type Responder: Responder;

    /// Spawns a new transceiver thread to communicate between this listener
    /// and all remote connections
    fn spawn<C, D>(
        self,
        sleep_duration: Duration,
        callback: C,
        err_callback: D,
    ) -> io::Result<TransceiverThread<DataAndAddr, ()>>
    where
        C: Fn(Vec<u8>, Self::Responder) + Send + 'static,
        D: Fn(Self::Error) -> bool + Send + 'static;
}
