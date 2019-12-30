pub mod bicrypter;
pub mod impls;

#[derive(Debug)]
pub enum Error {
    /// Contains generic AED error
    Aed(aead::Error),
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match &*self {
            Error::Aed(error) => write!(f, "AED: {:?}", error),
        }
    }
}
