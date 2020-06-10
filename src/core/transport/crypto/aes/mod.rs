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

use derive_more::{Display, Error};

#[derive(Debug, Display, Error)]
pub enum AesError {
    /// Contains generic AED error
    #[display(fmt = "{:?}", _0)]
    Generic(#[error(ignore)] aead::Error),
}

fn make_error_string(x: aead::Error) -> String {
    format!("{:?}", x)
}
