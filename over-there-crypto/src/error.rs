#[derive(Debug)]
pub enum Error {
    /// Internal Error related to encryption occurred
    EncryptFailed(Box<dyn std::error::Error>),

    /// Internal Error related to decryption occurred
    DecryptFailed(Box<dyn std::error::Error>),

    /// Contains the nonce that was already used
    NonceAlreadyUsed { nonce: Vec<u8> },

    /// Contains size of nonce provided
    NonceWrongSize { provided_size: usize },

    /// When a nonce was expected and none was provided
    MissingNonce,
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match &*self {
            Error::EncryptFailed(error) => write!(f, "Internal encrypt error: {:?}", error),
            Error::DecryptFailed(error) => write!(f, "Internal decrypt error: {:?}", error),
            Error::NonceAlreadyUsed { nonce } => write!(f, "Nonce already used: {:?}", nonce),
            Error::NonceWrongSize { provided_size } => {
                write!(f, "Nonce is wrong size: {}", provided_size)
            }
            Error::MissingNonce => write!(f, "Nonce expected but not provided"),
        }
    }
}
