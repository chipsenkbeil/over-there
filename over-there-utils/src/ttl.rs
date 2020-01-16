use std::cmp::Ordering;
use std::time::Duration;
use std::time::Instant;

/// Represents a value that has a limited lifetime before expiring
#[derive(Eq)]
pub struct TtlValue<T> {
    pub value: T,
    last_touched: Instant,
    ttl: Duration,
}

impl<T> TtlValue<T> {
    pub fn new(value: T, ttl: Duration) -> Self {
        Self {
            value,
            last_touched: Instant::now(),
            ttl,
        }
    }

    pub fn refresh(&mut self) {
        self.last_touched = Instant::now();
    }

    pub fn has_expired(&self) -> bool {
        self.last_touched.elapsed().checked_sub(self.ttl).is_some()
    }
}

impl<T: Eq> PartialOrd for TtlValue<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: Eq> Ord for TtlValue<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.last_touched.cmp(&other.last_touched)
    }
}

impl<T> PartialEq for TtlValue<T> {
    fn eq(&self, other: &Self) -> bool {
        self.last_touched == other.last_touched
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refresh_should_renew_lifetime_of_value() {
        let mut ttl_value = TtlValue::new(0, Duration::from_millis(5));

        std::thread::sleep(Duration::from_millis(3));

        ttl_value.refresh();

        std::thread::sleep(Duration::from_millis(3));

        assert!(!ttl_value.has_expired(), "Value expired unexpectedly");
    }

    #[test]
    fn has_expired_should_return_false_if_value_has_not_expired() {
        let ttl_value = TtlValue::new(0, Duration::from_millis(5));

        std::thread::sleep(Duration::from_millis(1));

        assert!(!ttl_value.has_expired(), "Value expired unexpectedly");
    }

    #[test]
    fn has_expired_should_return_true_if_value_has_expired() {
        let ttl_value = TtlValue::new(0, Duration::from_millis(5));

        std::thread::sleep(Duration::from_millis(6));

        assert!(ttl_value.has_expired(), "Value not expired when should be");
    }

    #[test]
    fn ordering_uses_last_touch() {
        let mut ttl_value_1 = TtlValue::new(0, Duration::from_millis(5));

        std::thread::sleep(Duration::from_millis(5));

        let ttl_value_2 = TtlValue::new(0, Duration::from_millis(5));

        assert!(ttl_value_1 < ttl_value_2);

        ttl_value_1.refresh();

        assert!(ttl_value_1 > ttl_value_2);
    }

    #[test]
    fn equality_uses_last_touch() {
        let ttl_value_1 = TtlValue::new(0, Duration::from_millis(5));

        std::thread::sleep(Duration::from_millis(5));

        let mut ttl_value_2 = TtlValue::new(0, Duration::from_millis(5));

        assert!(ttl_value_1 != ttl_value_2);

        ttl_value_2.last_touched = ttl_value_1.last_touched;

        assert!(ttl_value_1 == ttl_value_2);
    }
}
