pub mod cache;

use super::{AssociatedData, CryptError};
use rand::Rng;
use serde::{Deserialize, Serialize};

/// Represents a 96-bit nonce (12 bytes)
pub type Nonce96Bits = [u8; 12];

/// Represents a 128-bit nonce (16 bytes)
pub type Nonce128Bits = [u8; 16];

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum Nonce {
    Nonce96Bits(Nonce96Bits),
    Nonce128Bits(Nonce128Bits),
}

impl Nonce {
    /// Converts slice of bytes to a nonce if it is the right size,
    /// otherwise returns nothing
    pub fn from_slice(&self, slice: &[u8]) -> Option<Self> {
        use std::convert::TryInto;
        if slice.len() == NonceSize::Nonce96Bits.size_in_bytes() {
            slice.try_into().map(Self::Nonce96Bits).ok()
        } else if slice.len() == NonceSize::Nonce128Bits.size_in_bytes() {
            slice.try_into().map(Self::Nonce128Bits).ok()
        } else {
            None
        }
    }

    pub fn as_slice(&self) -> &[u8] {
        match self {
            Self::Nonce96Bits(nonce) => nonce,
            Self::Nonce128Bits(nonce) => nonce,
        }
    }
}

impl From<Nonce96Bits> for Nonce {
    fn from(buffer: Nonce96Bits) -> Self {
        Self::Nonce96Bits(buffer)
    }
}

impl From<Nonce128Bits> for Nonce {
    fn from(buffer: Nonce128Bits) -> Self {
        Self::Nonce128Bits(buffer)
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
/// Represents the size of a nonce
pub enum NonceSize {
    Nonce96Bits,
    Nonce128Bits,
}

impl NonceSize {
    pub fn size_in_bytes(&self) -> usize {
        match self {
            NonceSize::Nonce96Bits => 12,
            NonceSize::Nonce128Bits => 16,
        }
    }
}

impl From<Nonce> for NonceSize {
    fn from(nonce: Nonce) -> Self {
        match nonce {
            Nonce::Nonce96Bits(_) => Self::Nonce96Bits,
            Nonce::Nonce128Bits(_) => Self::Nonce128Bits,
        }
    }
}

impl From<NonceSize> for Nonce {
    fn from(nonce_size: NonceSize) -> Self {
        match nonce_size {
            NonceSize::Nonce96Bits => Self::Nonce96Bits(new_96bit_nonce()),
            NonceSize::Nonce128Bits => Self::Nonce128Bits(new_128bit_nonce()),
        }
    }
}

/// Validates the size of a nonce with a desired size
pub fn validate_nonce_size(nonce_size: NonceSize, desired_size: usize) -> Result<(), CryptError> {
    if desired_size != nonce_size.size_in_bytes() {
        return Err(CryptError::NonceWrongSize {
            provided_size: desired_size,
        });
    }

    Ok(())
}

/// Produces a 96-bit nonce (12 bytes)
pub fn new_96bit_nonce() -> Nonce96Bits {
    rand::thread_rng().gen::<Nonce96Bits>()
}

/// Produces a 128-bit nonce (16 bytes)
pub fn new_128bit_nonce() -> Nonce128Bits {
    rand::thread_rng().gen::<Nonce128Bits>()
}
