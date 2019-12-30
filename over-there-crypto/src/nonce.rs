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

/// Produces a dynamically-sized nonce
pub fn new_nonce_vec(nonce_size: NonceSize) -> Vec<u8> {
    match nonce_size {
        NonceSize::Nonce96Bits => new_96bit_nonce().to_vec(),
        NonceSize::Nonce128Bits => new_128bit_nonce().to_vec(),
    }
}

/// Converts nonce size to physical byte size
pub fn nonce_size_to_byte_length(nonce_size: NonceSize) -> usize {
    match nonce_size {
        NonceSize::Nonce96Bits => 12,
        NonceSize::Nonce128Bits => 16,
    }
}

/// Produces a 96-bit nonce (12 bytes)
pub fn new_96bit_nonce() -> Nonce96Bits {
    rand::thread_rng().gen::<Nonce96Bits>()
}

/// Produces a 128-bit nonce (16 bytes)
pub fn new_128bit_nonce() -> Nonce128Bits {
    rand::thread_rng().gen::<Nonce128Bits>()
}
