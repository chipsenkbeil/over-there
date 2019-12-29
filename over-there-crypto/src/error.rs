#[derive(Debug)]
pub enum Error {
    /// Internal Error related to encryption/decryption occurred
    /// *Bubble up source*
    Internal(Box<dyn std::error::Error>),
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match &*self {
            Error::Internal(error) => write!(f, "Internal implementation error: {:?}", error),
        }
    }
}
