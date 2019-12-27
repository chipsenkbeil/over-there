pub mod udp;

use crate::transmitter::{self, Transmitter};
use over_there_transport::Transport;

#[derive(Debug)]
pub enum Error {
    IO(std::io::Error),
    Transmitter(transmitter::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match &*self {
            Error::IO(error) => write!(f, "IO Error encountered during communication: {:?}", error),
            Error::Transmitter(error) => write!(
                f,
                "Msg Manager Error encountered during communication: {:?}",
                error
            ),
        }
    }
}

impl std::error::Error for Error {}

pub struct Communicator<T: Transport> {
    transmitter: Transmitter,
    pub transport: T,
}

impl<T: Transport> Communicator<T> {
    pub fn new(transmitter: Transmitter, transport: T) -> Self {
        Communicator {
            transmitter,
            transport,
        }
    }

    pub fn from_transport(transport: T, max_data_per_packet: u32) -> Self {
        Self::new(Transmitter::new(max_data_per_packet), transport)
    }
}
