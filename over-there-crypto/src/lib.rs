pub mod aead;
pub mod noop;

mod error;
pub use error::Error;

/// Can both encrypt and decrypt
pub trait Bicrypter: Encrypter + Decrypter {}

/// Capable of encrypting data
pub trait Encrypter {
    fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>, Error>;
}

/// Capable of decrypting data
pub trait Decrypter {
    fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>, Error>;
}
