pub mod bicrypter;
pub mod impls;

pub mod nonce {
    /// Represents a 96-bit nonce (12 bytes)
    pub type Type = [u8; 12];

    /// Produces a 96-bit nonce (12 bytes)
    pub fn new() -> Type {
        use rand::Rng;
        rand::thread_rng().gen::<Type>()
    }
}

#[derive(Debug)]
pub enum Error {
    Aed(aead::Error),
    NonceAlreadyUsed(nonce::Type),
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            Error::Aed(error) => write!(f, "AED: {:?}", error),
            Error::NonceAlreadyUsed(nonce) => write!(f, "Nonce already used: {:?}", nonce),
        }
    }
}
