use rand::Rng;
use serde::{Deserialize, Serialize};

pub type Key128Bits = [u8; 16];
pub type Key256Bits = [u8; 32];
pub type Key512Bits = [u8; 64];

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum KeySize {
    Key128Bits,
    Key256Bits,
    Key512Bits,
}

impl From<KeySize> for usize {
    fn from(key_size: KeySize) -> Self {
        match key_size {
            KeySize::Key128Bits => 16,
            KeySize::Key256Bits => 32,
            KeySize::Key512Bits => 64,
        }
    }
}

impl From<KeySize> for Vec<u8> {
    fn from(key_size: KeySize) -> Self {
        match key_size {
            KeySize::Key128Bits => new_128bit_key().to_vec(),
            KeySize::Key256Bits => new_256bit_key().to_vec(),
            KeySize::Key512Bits => new_512bit_key().to_vec(),
        }
    }
}

pub fn new_128bit_key() -> Key128Bits {
    let mut buffer = [0; 16];
    rand::thread_rng().fill(&mut buffer);
    buffer
}

pub fn new_256bit_key() -> Key256Bits {
    let mut buffer = [0; 32];
    rand::thread_rng().fill(&mut buffer);
    buffer
}

pub fn new_512bit_key() -> Key512Bits {
    let mut buffer = [0; 64];
    rand::thread_rng().fill(&mut buffer);
    buffer
}
