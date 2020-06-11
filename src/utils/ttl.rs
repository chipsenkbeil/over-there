use std::cmp::Ordering;
use std::hash::{Hash, Hasher};
use std::ops::{Deref, DerefMut};
use std::time::Duration;
use std::time::Instant;

/// Represents a value that has a limited lifetime before expiring
#[derive(Debug)]
pub struct TtlValue<T> {
    pub value: T,
    last_touched: Instant,
    ttl: Duration,
}

/// Represents a void value, purely used to keep track of access times
pub type EmptyTtlValue = TtlValue<()>;

impl EmptyTtlValue {
    pub fn empty(ttl: Duration) -> Self {
        Self::new((), ttl)
    }
}

impl<T> TtlValue<T> {
    pub fn new(value: T, ttl: Duration) -> Self {
        Self {
            value,
            last_touched: Instant::now(),
            ttl,
        }
    }

    pub fn touch(&mut self) {
        self.last_touched = Instant::now();
    }

    pub fn last_touched(&self) -> &Instant {
        &self.last_touched
    }

    pub fn ttl(&self) -> &Duration {
        &self.ttl
    }

    pub fn has_expired(&self) -> bool {
        self.last_touched.elapsed().checked_sub(self.ttl).is_some()
    }
}

impl<T: Hash> Hash for TtlValue<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.value.hash(state);
    }
}

impl<T: Eq> Eq for TtlValue<T> {}

impl<T: Eq> PartialEq for TtlValue<T> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<T: PartialOrd + Eq> PartialOrd for TtlValue<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.value.partial_cmp(&other.value)
    }
}

impl<T: Ord> Ord for TtlValue<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.value.cmp(&other.value)
    }
}

impl<T: Clone> Clone for TtlValue<T> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            last_touched: self.last_touched,
            ttl: self.ttl,
        }
    }
}

impl<T: Copy> Copy for TtlValue<T> {}

impl<T> From<T> for TtlValue<T> {
    fn from(value: T) -> Self {
        Self::new(value, Duration::new(0, 0))
    }
}

impl<T> Into<Instant> for TtlValue<T> {
    fn into(self) -> Instant {
        self.last_touched
    }
}

impl<T> Into<Duration> for TtlValue<T> {
    fn into(self) -> Duration {
        self.ttl
    }
}

impl<T> Deref for TtlValue<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T> DerefMut for TtlValue<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn touch_should_renew_lifetime_of_value() {
        let mut ttl_value = TtlValue::new(0, Duration::from_millis(100));

        // Make the last touched time be in the past where we exceed the TTL
        ttl_value.last_touched = Instant::now()
            .checked_sub(Duration::from_millis(1000))
            .unwrap();

        // Now refresh
        ttl_value.touch();

        assert!(!ttl_value.has_expired(), "Value expired unexpectedly");
    }

    #[test]
    fn has_expired_should_return_false_if_value_has_not_expired() {
        let ttl_value = TtlValue::new(0, Duration::from_millis(100));

        assert!(!ttl_value.has_expired(), "Value expired unexpectedly");
    }

    #[test]
    fn has_expired_should_return_true_if_value_has_expired() {
        let mut ttl_value = TtlValue::new(0, Duration::from_millis(100));

        // Make the last touched time be in the past where we exceed the TTL
        ttl_value.last_touched = Instant::now()
            .checked_sub(Duration::from_millis(1000))
            .unwrap();

        assert!(ttl_value.has_expired(), "Value not expired when should be");
    }

    #[test]
    fn hash_should_use_underlying_value() {
        // Values do not overlap, so creates two entries
        let mut set = HashSet::new();
        let v1 = TtlValue::new(0, Duration::from_millis(5));
        let v2 = TtlValue::new(1, Duration::from_millis(5));

        set.insert(v1);
        set.insert(v2);

        assert_eq!(set.len(), 2);
        assert!(set.get(&0.into()).is_some());
        assert!(set.get(&1.into()).is_some());

        // Values do overlap, so creates one entry
        let mut set = HashSet::new();
        let v1 = TtlValue::new(2, Duration::from_millis(5));
        let v2 = TtlValue::new(2, Duration::from_millis(10));

        set.insert(v1);
        set.insert(v2);

        assert_eq!(set.len(), 1);
        assert!(set.get(&2.into()).is_some());
    }

    #[test]
    fn partial_eq_should_use_underlying_value() {
        let v1 = TtlValue::new(5, Duration::from_millis(1));
        let v2 = TtlValue::new(5, Duration::from_millis(2));

        assert_eq!(v1, v2);
    }

    #[test]
    fn partial_ord_should_use_underlying_value() {
        let v1 = TtlValue::new(3, Duration::from_millis(1));
        let v2 = TtlValue::new(5, Duration::from_millis(1));

        assert!(v1 < v2);
    }

    #[test]
    fn clone_should_use_underlying_value() {
        let v1 = TtlValue::new(3, Duration::from_millis(1));

        assert_eq!(v1, v1.clone());
    }

    #[test]
    fn copy_should_use_underlying_value() {
        let v1 = TtlValue::new(3, Duration::from_millis(1));
        let v2 = v1;

        assert_eq!(v1, v2);
    }

    #[test]
    fn from_should_use_underlying_value_and_produce_a_duration_of_zero() {
        let v1 = TtlValue::from(3);

        assert_eq!(v1.value, 3);
        assert_eq!(v1.ttl, Duration::new(0, 0));
    }

    #[test]
    fn into_should_return_last_touched_if_type_is_instant() {
        let v1 = TtlValue::new(3, Duration::from_millis(5));
        let v2 = v1.clone();

        let i: Instant = v1.into();
        assert_eq!(i, v2.last_touched);
    }

    #[test]
    fn into_should_return_ttl_if_type_is_duration() {
        let v1 = TtlValue::new(3, Duration::from_millis(5));

        let d: Duration = v1.into();
        assert_eq!(d, Duration::from_millis(5));
    }

    #[test]
    fn deref_should_yield_underlying_value() {
        let v1 = TtlValue::new(5, Duration::from_millis(1));

        assert_eq!(*v1, 5);
    }

    #[test]
    fn deref_mut_should_yield_underlying_value() {
        let mut v1 = TtlValue::new(5, Duration::from_millis(1));

        *v1 = 10;

        assert_eq!(*v1, 10);
    }
}
