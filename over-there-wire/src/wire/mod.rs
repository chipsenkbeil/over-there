mod input;
mod output;
mod packet;

use std::time::Duration;

// Export errors
pub use input::assembler::AssemblerError;
pub use input::{InputProcessor, InputProcessorError};
pub use output::disassembler::DisassemblerError;
pub use output::{OutputProcessor, OutputProcessorError};

// Re-export the auth and crypto interfaces
pub use over_there_auth::{Signer, Verifier};
pub use over_there_crypto::{Decrypter, Encrypter};

pub struct Wire<S, V, E, D>
where
    S: Signer,
    V: Verifier,
    E: Encrypter,
    D: Decrypter,
{
    /// Processes input coming into the wire
    pub input: InputProcessor<V, D>,

    /// Processes output leaving on the wire
    pub output: OutputProcessor<S, E>,
}

impl<S, V, E, D> Wire<S, V, E, D>
where
    S: Signer,
    V: Verifier,
    E: Encrypter,
    D: Decrypter,
{
    pub fn new(
        transmission_size: usize,
        packet_ttl: Duration,
        signer: S,
        verifier: V,
        encrypter: E,
        decrypter: D,
    ) -> Self {
        let input = InputProcessor::new(packet_ttl, verifier, decrypter);
        let output = OutputProcessor::new(transmission_size, signer, encrypter);
        Self { input, output }
    }
}
