use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum Either<T, U> {
    Left(T),
    Right(U),
}

impl<T, U> Either<T, U> {
    pub fn is_left(&self) -> bool {
        self.get_left().is_some()
    }

    pub fn is_right(&self) -> bool {
        self.get_right().is_some()
    }

    pub fn get_left(&self) -> Option<&T> {
        match self {
            Self::Left(x) => Some(x),
            _ => None,
        }
    }

    pub fn get_right(&self) -> Option<&U> {
        match self {
            Self::Right(x) => Some(x),
            _ => None,
        }
    }
}
