#[derive(Debug)]
pub enum Error {
    /// Internal Error related to encryption occurred
    Encrypt(Box<dyn std::error::Error>),

    /// Internal Error related to decryption occurred
    Decrypt(Box<dyn std::error::Error>),
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match &*self {
            Error::Encrypt(error) => write!(f, "Internal encrypt error: {:?}", error),
            Error::Decrypt(error) => write!(f, "Internal decrypt error: {:?}", error),
        }
    }
}
