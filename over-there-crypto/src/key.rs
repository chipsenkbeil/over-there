use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_big_array::big_array;

pub type Key128Bits = [u8; 16];
pub type Key256Bits = [u8; 32];
pub type Key512Bits = [u8; 64];

big_array! { BigArray; }

#[derive(Serialize, Deserialize, Clone, Copy)]
pub enum Key {
    Key128Bits(Key128Bits),
    Key256Bits(Key256Bits),

    #[serde(with = "BigArray")]
    Key512Bits(Key512Bits),
}

impl Key {
    /// Converts slice of bytes to a nonce if it is the right size,
    /// otherwise returns nothing
    pub fn from_slice(&self, slice: &[u8]) -> Option<Self> {
        use std::convert::TryInto;
        if slice.len() == KeySize::Key128Bits.size_in_bytes() {
            slice.try_into().map(Self::Key128Bits).ok()
        } else if slice.len() == KeySize::Key256Bits.size_in_bytes() {
            slice.try_into().map(Self::Key256Bits).ok()
        } else if slice.len() == KeySize::Key512Bits.size_in_bytes() {
            // NOTE: 64-byte array requires special handling due to
            //       limitations in rust right now
            let mut key = [0; 64];
            key.copy_from_slice(slice);
            Some(Self::Key512Bits(key))
        } else {
            None
        }
    }

    pub fn as_slice(&self) -> &[u8] {
        match self {
            Self::Key128Bits(nonce) => nonce,
            Self::Key256Bits(nonce) => nonce,
            Self::Key512Bits(nonce) => nonce,
        }
    }
}

impl std::fmt::Debug for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Key128Bits(k) => write!(f, "Key {:?}", k),
            Self::Key256Bits(k) => write!(f, "Key {:?}", k),
            Self::Key512Bits(k) => {
                let k_str = k
                    .iter()
                    .map(|n| n.to_string())
                    .collect::<Vec<String>>()
                    .join(",");
                write!(f, "Key [{:?}]", k_str)
            }
        }
    }
}

impl From<Key128Bits> for Key {
    fn from(buffer: Key128Bits) -> Self {
        Self::Key128Bits(buffer)
    }
}

impl From<Key256Bits> for Key {
    fn from(buffer: Key256Bits) -> Self {
        Self::Key256Bits(buffer)
    }
}

impl From<Key512Bits> for Key {
    fn from(buffer: Key512Bits) -> Self {
        Self::Key512Bits(buffer)
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum KeySize {
    Key128Bits,
    Key256Bits,
    Key512Bits,
}

impl KeySize {
    pub fn size_in_bytes(&self) -> usize {
        match self {
            KeySize::Key128Bits => 16,
            KeySize::Key256Bits => 32,
            KeySize::Key512Bits => 64,
        }
    }
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
