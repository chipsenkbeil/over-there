use std::sync::RwLock;

pub struct Capture<T> {
    value: RwLock<Option<T>>,
}

impl<T> Capture<T> {
    pub fn take(&self) -> Option<T> {
        let mut x = self.value.write().unwrap();
        x.take()
    }

    pub fn set(&self, value: T) {
        let mut x = self.value.write().unwrap();
        *x = Some(value);
    }
}

impl<T> Default for Capture<T> {
    fn default() -> Self {
        Self::from(None)
    }
}

impl<T> From<T> for Capture<T> {
    fn from(x: T) -> Self {
        Self::from(Some(x))
    }
}

impl<T> From<Option<T>> for Capture<T> {
    fn from(x: Option<T>) -> Self {
        Self {
            value: RwLock::new(x),
        }
    }
}
