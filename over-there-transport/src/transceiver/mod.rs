pub mod net;
pub mod receiver;
pub mod transmitter;

use crate::assembler::Assembler;
use crate::disassembler::Disassembler;
use over_there_auth::{Signer, Verifier};
use over_there_crypto::{Decrypter, Encrypter};
use receiver::ReceiverContext;
use transmitter::TransmitterContext;

pub(crate) struct Context<A, B>
where
    A: Signer + Verifier,
    B: Encrypter + Decrypter,
{
    /// Buffer to contain bytes for temporary storage
    /// NOTE: Heap allocation until we can control array size with const generics
    buffer: Box<[u8]>,

    /// Maximum size at which to send data
    transmission_size: usize,

    /// Assembler used to gather packets together
    assembler: Assembler,

    /// Disassembler used to break up data into packets
    disassembler: Disassembler,

    /// Performs signing/verification on data
    authenticator: A,

    /// Performs encryption/decryption on data
    bicrypter: B,
}

impl<A, B> Context<A, B>
where
    A: Signer + Verifier,
    B: Encrypter + Decrypter,
{
    pub fn new(transmission_size: usize, authenticator: A, bicrypter: B) -> Self {
        Self {
            buffer: vec![0; transmission_size].into_boxed_slice(),
            transmission_size,
            assembler: Assembler::new(),
            disassembler: Disassembler::new(),
            authenticator,
            bicrypter,
        }
    }
}

impl<'a, A, B> From<&'a mut Context<A, B>> for ReceiverContext<'a, A, B>
where
    A: Signer + Verifier,
    B: Encrypter + Decrypter,
{
    fn from(ctx: &'a mut Context<A, B>) -> Self {
        Self {
            buffer: &mut ctx.buffer,
            assembler: &mut ctx.assembler,
            verifier: &ctx.authenticator,
            decrypter: &ctx.bicrypter,
        }
    }
}

impl<'a, A, B> From<&'a mut Context<A, B>> for TransmitterContext<'a, A, B>
where
    A: Signer + Verifier,
    B: Encrypter + Decrypter,
{
    fn from(ctx: &'a mut Context<A, B>) -> Self {
        Self {
            transmission_size: ctx.transmission_size,
            disassembler: &mut ctx.disassembler,
            signer: &ctx.authenticator,
            encrypter: &ctx.bicrypter,
        }
    }
}
