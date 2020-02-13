mod input;
mod output;
mod packet;

use over_there_derive::Error;
use std::future::Future;
use std::io;
use std::net::SocketAddr;
use std::time::Duration;

// Export errors
pub use input::assembler::AssemblerError;
pub use input::{InputProcessor, InputProcessorError};
pub use output::disassembler::DisassemblerError;
pub use output::{OutputProcessor, OutputProcessorError};

// Re-export the auth and crypto interfaces
pub use over_there_auth::{Signer, Verifier};
pub use over_there_crypto::{Decrypter, Encrypter};

#[derive(Debug, Error)]
pub enum InboundWireError {
    IO(io::Error),
    InputProcessor(InputProcessorError),
}

#[derive(Debug, Error)]
pub enum OutboundWireError {
    IO(io::Error),
    OutputProcessor(OutputProcessorError),
}

/// Wire for inbound communication
pub struct InboundWire<V, D>
where
    V: Verifier,
    D: Decrypter,
{
    /// Processes input coming into the wire
    input_processor: InputProcessor<V, D>,

    /// Maximum size to expect data
    transmission_size: usize,
}

impl<V, D> InboundWire<V, D>
where
    V: Verifier,
    D: Decrypter,
{
    pub fn new(transmission_size: usize, packet_ttl: Duration, verifier: V, decrypter: D) -> Self {
        let input_processor = InputProcessor::new(packet_ttl, verifier, decrypter);
        Self {
            input_processor,
            transmission_size,
        }
    }

    /// Receives data synchronously using the provided function
    pub fn recv<F>(&mut self, mut f: F) -> Result<Option<(Vec<u8>, SocketAddr)>, InboundWireError>
    where
        F: FnMut(&mut [u8]) -> io::Result<(usize, SocketAddr)>,
    {
        let mut tmp_buf = vec![0; self.transmission_size];

        let (size, addr) = f(&mut tmp_buf).map_err(InboundWireError::IO)?;
        self.input_processor
            .process(&tmp_buf[..size])
            .map(|opt| opt.map(|data| (data, addr)))
            .map_err(InboundWireError::InputProcessor)
    }

    /// Receives data asynchronously using the provided function
    pub async fn async_recv<F, R>(
        &mut self,
        mut f: F,
    ) -> Result<Option<(Vec<u8>, SocketAddr)>, InboundWireError>
    where
        F: FnMut(&mut [u8]) -> R,
        R: Future<Output = io::Result<(usize, SocketAddr)>>,
    {
        let mut tmp_buf = vec![0; self.transmission_size];

        let (size, addr) = f(&mut tmp_buf).await.map_err(InboundWireError::IO)?;
        self.input_processor
            .process(&tmp_buf[..size])
            .map(|opt| opt.map(|data| (data, addr)))
            .map_err(InboundWireError::InputProcessor)
    }
}

/// Wire for outbound communication
pub struct OutboundWire<S, E>
where
    S: Signer,
    E: Encrypter,
{
    /// Processes output leaving on the wire
    output_processor: OutputProcessor<S, E>,
}

impl<S, E> OutboundWire<S, E>
where
    S: Signer,
    E: Encrypter,
{
    pub fn new(transmission_size: usize, signer: S, encrypter: E) -> Self {
        let output_processor = OutputProcessor::new(transmission_size, signer, encrypter);
        Self { output_processor }
    }

    /// Sends data in buf synchronously using the provided function
    pub fn send<F>(&mut self, buf: &[u8], mut f: F) -> Result<(), OutboundWireError>
    where
        F: FnMut(&[u8]) -> io::Result<()>,
    {
        let data = self
            .output_processor
            .process(buf)
            .map_err(OutboundWireError::OutputProcessor)?;

        for packet_bytes in data.iter() {
            f(packet_bytes).map_err(OutboundWireError::IO)?;
        }

        Ok(())
    }

    /// Sends data in buf asynchronously using the provided function
    pub async fn async_send<F, R>(&mut self, buf: &[u8], mut f: F) -> Result<(), OutboundWireError>
    where
        F: FnMut(&[u8]) -> R,
        R: Future<Output = io::Result<()>>,
    {
        let data = self
            .output_processor
            .process(buf)
            .map_err(OutboundWireError::OutputProcessor)?;

        for packet_bytes in data.iter() {
            f(packet_bytes).await.map_err(OutboundWireError::IO)?;
        }

        Ok(())
    }
}
