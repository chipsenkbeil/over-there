mod assembler;
mod disassembler;
pub mod net;
mod packet;
mod transceiver;

pub mod constants {
    pub const DEFAULT_TTL_IN_SECS: u64 = 60 * 5;
}

pub use assembler::AssemblerError;
pub use disassembler::DisassemblerError;
pub use transceiver::net::{
    tcp::{listener::TcpListenerTransceiver, stream::TcpStreamTransceiver},
    udp::{stream::UdpStreamTransceiver, UdpTransceiver},
    AddrNetResponder, NetListener, NetResponder, NetStream, NetTransmission,
};
pub use transceiver::receiver::ReceiverError;
pub use transceiver::transmitter::TransmitterError;
pub use transceiver::{
    Responder, ResponderError, TransceiverContext, TransceiverThread, TransceiverThreadError,
};
