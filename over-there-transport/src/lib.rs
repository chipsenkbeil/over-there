mod assembler;
mod disassembler;
pub mod net;
mod packet;
mod transceiver;

pub mod constants {
    use std::time::Duration;

    /// 5 minute default TTL
    pub const DEFAULT_TTL: Duration = Duration::from_secs(60 * 5);
}

pub use assembler::AssemblerError;
pub use disassembler::DisassemblerError;
pub use transceiver::net::{
    tcp::{
        listener::{TcpListenerTransceiver, TcpListenerTransceiverError},
        stream::{TcpStreamTransceiver, TcpStreamTransceiverError},
    },
    udp::{
        stream::{UdpStreamTransceiver, UdpStreamTransceiverError},
        UdpTransceiver, UdpTransceiverError,
    },
    AddrNetResponder, NetListener, NetResponder, NetStream, NetTransmission,
};
pub use transceiver::receiver::ReceiverError;
pub use transceiver::transmitter::TransmitterError;
pub use transceiver::{
    Responder, ResponderError, TransceiverContext, TransceiverThread, TransceiverThreadError,
};
