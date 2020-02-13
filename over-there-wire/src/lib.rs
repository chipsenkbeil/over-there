pub mod net;
mod wire;

pub mod constants {
    use std::time::Duration;

    /// 5 minute default TTL
    pub const DEFAULT_TTL: Duration = Duration::from_secs(60 * 5);
}

// Export errors
pub use wire::{
    AssemblerError, DisassemblerError, InboundWireError, InputProcessorError, OutboundWireError,
    OutputProcessorError,
};

// Export useful constructs
pub use net::NetTransmission;
pub use wire::{InboundWire, OutboundWire};

// Re-export the auth and crypto interfaces
pub use over_there_auth::{Signer, Verifier};
pub use over_there_crypto::{Decrypter, Encrypter};
