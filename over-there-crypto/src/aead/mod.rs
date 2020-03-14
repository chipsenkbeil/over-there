mod aes_gcm_128;
mod aes_gcm_256;
mod aes_gcm_siv_128;
mod aes_gcm_siv_256;
mod aes_siv_128;
mod aes_siv_256;

pub use aes_gcm_128::Aes128GcmBicrypter;
pub use aes_gcm_256::Aes256GcmBicrypter;
pub use aes_gcm_siv_128::Aes128GcmSivBicrypter;
pub use aes_gcm_siv_256::Aes256GcmSivBicrypter;
pub use aes_siv_128::Aes128SivBicrypter;
pub use aes_siv_256::Aes256SivBicrypter;

use over_there_derive::Error;

#[derive(Debug, Error)]
pub enum AeadError {
    /// Contains generic AED error
    Generic(aead::Error),
}

fn make_error_string(x: aead::Error) -> String {
    format!("{:?}", x)
}
