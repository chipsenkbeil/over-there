pub mod bicrypter;
pub mod impls;

pub mod nonce {
    /// Represents a 96-bit nonce (12 bytes)
    pub type Length96Bits = [u8; 12];

    /// Represents a 128-bit nonce (16 bytes)
    pub type Length128Bits = [u8; 16];

    #[derive(Copy, Clone)]
    /// Represents the size of a nonce
    pub enum Size {
        Length96Bits,
        Length128Bits,
    }

    /// Produces a dynamically-sized nonce
    pub fn new(size: Size) -> Vec<u8> {
        match size {
            Size::Length96Bits => new_96bit().to_vec(),
            Size::Length128Bits => new_128bit().to_vec(),
        }
    }

    /// Converts enum to physical byte size
    pub fn size_to_byte_length(size: Size) -> usize {
        match size {
            Size::Length96Bits => 12,
            Size::Length128Bits => 16,
        }
    }

    /// Produces a 96-bit nonce (12 bytes)
    pub fn new_96bit() -> Length96Bits {
        use rand::Rng;
        rand::thread_rng().gen::<Length96Bits>()
    }

    /// Produces a 128-bit nonce (16 bytes)
    pub fn new_128bit() -> Length128Bits {
        use rand::Rng;
        rand::thread_rng().gen::<Length128Bits>()
    }
}

#[derive(Debug)]
pub enum Error {
    /// Contains generic AED error
    Aed(aead::Error),

    /// Contains the nonce that was already used
    NonceAlreadyUsed(Vec<u8>),

    /// Contains size of nonce provided
    NonceWrongSize(usize),
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match &*self {
            Error::Aed(error) => write!(f, "AED: {:?}", error),
            Error::NonceAlreadyUsed(nonce) => write!(f, "Nonce already used: {:?}", nonce),
            Error::NonceWrongSize(size) => write!(f, "Nonce is wrong size: {}", size),
        }
    }
}
