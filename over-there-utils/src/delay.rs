use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

/// Represents a delayed execution of a function
pub struct Delay {
    should_cancel: Arc<AtomicBool>,
    thread_handle: JoinHandle<()>,
}

impl Delay {
    /// Spawns a new thread that will invoke the provided function after the
    /// given timeout has been exceeded. There is no guarantee that the
    /// function will be executed exactly on the given time, only that it will
    /// be executed no earlier than until the specified duration has elapsed
    pub fn spawn<F, T>(timeout: Duration, f: F) -> Self
    where
        F: FnOnce() -> T + Send + 'static,
    {
        let should_cancel = Arc::new(AtomicBool::new(false));
        let should_cancel_2 = Arc::clone(&should_cancel);

        let start_time = Instant::now();
        let thread_handle = thread::spawn(move || {
            let mut timeout_remaining = timeout;
            loop {
                thread::park_timeout(timeout_remaining);
                let elapsed = start_time.elapsed();
                if elapsed >= timeout || should_cancel_2.load(Ordering::Acquire)
                {
                    break;
                }
                timeout_remaining = timeout - elapsed;
            }

            if !should_cancel_2.load(Ordering::Acquire) {
                f();
            }
        });

        Self {
            should_cancel,
            thread_handle,
        }
    }

    /// Cancels the delayed execution, if it has not yet occurred
    pub fn cancel(&self) {
        self.should_cancel.store(true, Ordering::Release);
        self.thread_handle.thread().unpark();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[test]
    fn delay_should_occur_on_or_after_timeout() {
        let delay_duration = Duration::from_millis(10);
        let before = Instant::now();

        let after = Arc::new(Mutex::new(Instant::now()));
        let after_delay = Arc::clone(&after);
        let _delay = Delay::spawn(delay_duration, move || {
            *after_delay.lock().unwrap() = Instant::now();
        });

        // Wait twice as long to ensure the delay happens
        thread::sleep(delay_duration * 2);

        let elapsed = after
            .lock()
            .unwrap()
            .checked_duration_since(before)
            .unwrap();
        assert!(
            elapsed >= delay_duration,
            "Delay did not occur after {:?}",
            delay_duration
        );
    }

    #[test]
    fn delay_should_occur_even_if_instance_is_dropped() {
        let delay_duration = Duration::from_millis(10);
        let did_occur = Arc::new(AtomicBool::new(false));
        let did_occur_2 = Arc::clone(&did_occur);
        let delay = Delay::spawn(delay_duration, move || {
            did_occur_2.store(true, Ordering::Release);
        });

        // Immediately drop the delay instance
        drop(delay);

        // Wait twice as long to ensure the delay happens
        thread::sleep(delay_duration * 2);

        assert!(
            did_occur.load(Ordering::Acquire),
            "Delayed call did not occur",
        );
    }

    #[test]
    fn delay_should_not_occur_if_cancelled() {
        let delay_duration = Duration::from_millis(10);
        let did_occur = Arc::new(AtomicBool::new(false));
        let did_occur_2 = Arc::clone(&did_occur);
        let delay = Delay::spawn(delay_duration, move || {
            did_occur_2.store(true, Ordering::Release);
        });

        // Cancel immediately
        delay.cancel();

        // Wait twice as long to ensure the delay happens
        thread::sleep(delay_duration * 2);

        assert!(
            !did_occur.load(Ordering::Acquire),
            "Delay occurred unexpectedly",
        );
    }

    #[test]
    fn delay_cancel_should_do_nothing_if_delay_already_occurred() {
        let delay_duration = Duration::from_millis(10);
        let did_occur = Arc::new(AtomicBool::new(false));
        let did_occur_2 = Arc::clone(&did_occur);
        let delay = Delay::spawn(delay_duration, move || {
            did_occur_2.store(true, Ordering::Release);
        });

        // Wait twice as long to ensure the delay happens
        thread::sleep(delay_duration * 2);

        // Cancel later, after the delay should have happened
        delay.cancel();

        assert!(
            did_occur.load(Ordering::Acquire),
            "Delayed call did not occur",
        );
    }
}
