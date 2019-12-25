use super::msg::manager::{MsgManager, MsgManangerError};
use super::transport::Transport;

#[derive(Debug)]
pub enum Error {
    IO(std::io::Error),
    MsgManager(MsgManangerError),
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
    msg_manager: MsgManager,
    transport: T,
}

impl<T: Transport> Communicator<T> {
    pub fn new(msg_manager: MsgManager, transport: T) -> Self {
        Communicator {
            msg_manager,
            transport,
        }
    }

    pub fn from_transport(transport: T, max_data_per_packet: u32) -> Self {
        Self::new(MsgManager::new(max_data_per_packet), transport)
    }

    pub fn transport(&self) -> &T {
        &self.transport
    }

    pub fn msg_manager(&self) -> &MsgManager {
        &self.msg_manager
    }
}
