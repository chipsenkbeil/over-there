use super::{manager, msg, transport};
use transport::Transport;

#[derive(Debug)]
pub enum Error {
    IO(std::io::Error),
    MsgManager(manager::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match &*self {
            Error::IO(error) => write!(f, "IO Error encountered during communication: {:?}", error),
            Error::MsgManager(error) => write!(
                f,
                "Msg Manager Error encountered during communication: {:?}",
                error
            ),
        }
    }
}

impl std::error::Error for Error {}

pub struct Communicator<T: Transport> {
    msg_manager: manager::MsgManager,
    transport: T,
}

impl<T: Transport> Communicator<T> {
    pub fn new(msg_manager: manager::MsgManager, transport: T) -> Self {
        Communicator {
            msg_manager,
            transport,
        }
    }

    pub fn from_transport(transport: T, max_data_per_packet: u32) -> Self {
        Self::new(manager::MsgManager::new(max_data_per_packet), transport)
    }

    pub fn transport(&self) -> &T {
        &self.transport
    }

    pub fn msg_manager(&self) -> &manager::MsgManager {
        &self.msg_manager
    }
}

impl<T: transport::net::NetworkTransport<transport::net::udp::UDP>> Communicator<T> {
    pub fn send(&self, msg: msg::Msg, addr: std::net::SocketAddr) -> Result<(), Error> {
        self.msg_manager()
            .send(msg, |data| {
                let _result = self.transport().send(data, addr)?;
                Ok(())
            })
            .map_err(Error::MsgManager)
    }

    pub fn recv(&self) -> Result<Option<(msg::Msg, std::net::SocketAddr)>, Error> {
        let mut addr: Option<std::net::SocketAddr> = None;
        let msg = self
            .msg_manager()
            .recv(|buf| {
                let (size, src) = self.transport().recv(buf)?;
                addr = Some(src);
                Ok(size)
            })
            .map_err(Error::MsgManager)?;
        Ok(match (msg, addr) {
            (Some(m), Some(a)) => Some((m, a)),
            _ => None,
        })
    }
}
