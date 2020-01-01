pub mod cache;

use super::{AssociatedData, CryptError};
use rand::Rng;
use serde::{Deserialize, Serialize};

/// Represents a 96-bit nonce (12 bytes)
pub type Nonce96Bits = [u8; 12];

/// Represents a 128-bit nonce (16 bytes)
pub type Nonce128Bits = [u8; 16];

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
/// Represents the size of a nonce
pub enum NonceSize {
    Nonce96Bits,
    Nonce128Bits,
}

impl From<NonceSize> for usize {
    fn from(nonce_size: NonceSize) -> Self {
        match nonce_size {
            NonceSize::Nonce96Bits => 12,
            NonceSize::Nonce128Bits => 16,
        }
    }
}

impl From<NonceSize> for Vec<u8> {
    fn from(nonce_size: NonceSize) -> Self {
        match nonce_size {
            NonceSize::Nonce96Bits => new_96bit_nonce().to_vec(),
            NonceSize::Nonce128Bits => new_128bit_nonce().to_vec(),
        }
    }
}

/// Validates the size of a nonce with a desired size
pub fn validate_nonce_size(nonce_size: NonceSize, desired_size: usize) -> Result<(), CryptError> {
    if desired_size != From::from(nonce_size) {
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
