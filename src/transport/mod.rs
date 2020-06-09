pub mod auth;
pub mod net;
mod wire;

pub mod constants {
    use std::time::Duration;

    /// 5 minute default TTL
    pub const DEFAULT_TTL: Duration = Duration::from_secs(60 * 5);
}

// Export errors
pub use wire::{
    DecoderError, EncoderError, InboundWireError, InputProcessorError,
    OutboundWireError, OutputProcessorError,
};

// Export useful constructs
pub use net::NetTransmission;
pub use wire::{
    tcp::{TcpStreamInboundWire, TcpStreamOutboundWire, TcpStreamWire},
    udp::{UdpSocketInboundWire, UdpSocketOutboundWire, UdpSocketWire},
    InboundWire, OutboundWire, Wire,
};

// Re-export the auth and crypto interfaces
pub use auth::{Authenticator, Signer, Verifier};
pub use over_there_crypto::{self as crypto, Bicrypter, Decrypter, Encrypter};
