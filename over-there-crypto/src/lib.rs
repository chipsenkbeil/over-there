#[cfg(any(
    feature = "include-aes-gcm",
    feature = "include-aes-gcm-siv",
    feature = "include-aes-siv"
))]
/// Provide AEAD-oriented crypto if we are using any of the associated features
pub mod aead;

/// Provides no-op implementations for encryption/decryption
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
