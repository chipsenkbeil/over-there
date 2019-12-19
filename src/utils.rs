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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_left_yields_true_if_left() {
        let e = Either::Left::<u32, &str>(123);
        assert_eq!(e.is_left(), true);
    }

    #[test]
    fn is_left_yields_false_if_right() {
        let e = Either::Right::<u32, &str>("123");
        assert_eq!(e.is_left(), false);
    }

    #[test]
    fn is_right_yields_true_if_right() {
        let e = Either::Right::<u32, &str>("123");
        assert_eq!(e.is_right(), true);
    }

    #[test]
    fn is_right_yields_false_if_left() {
        let e = Either::Left::<u32, &str>(123);
        assert_eq!(e.is_right(), false);
    }

    #[test]
    fn get_left_yields_some_value_if_left() {
        let e = Either::Left::<u32, &str>(123);
        assert_eq!(e.get_left(), Some(&123));
    }

    #[test]
    fn get_left_yields_none_if_right() {
        let e = Either::Right::<u32, &str>("123");
        assert_eq!(e.get_left(), None);
    }

    #[test]
    fn get_right_yields_some_value_if_right() {
        let e = Either::Right::<u32, &str>("123");
        assert_eq!(e.get_right(), Some(&"123"));
    }

    #[test]
    fn get_right_yields_none_if_left() {
        let e = Either::Left::<u32, &str>(123);
        assert_eq!(e.get_right(), None);
    }
}
