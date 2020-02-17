use over_there_derive::Error;
use std::time::{Duration, SystemTime, SystemTimeError};

#[derive(Debug, Error)]
pub enum Error {
    Exec(Box<dyn std::error::Error>),
    SystemTime(SystemTimeError),
    Timeout(Duration),
}

/// Invokes a function repeatedly until it yields Some(T); if a timeout is reached,
/// the function will return an error
pub fn loop_timeout<T>(
    timeout: Duration,
    mut f: impl FnMut() -> Result<Option<T>, Box<dyn std::error::Error>>,
) -> Result<T, Error> {
    let start_time = SystemTime::now();
    let mut result = None;
    while SystemTime::now()
        .duration_since(start_time)
        .map_err(Error::SystemTime)?
        < timeout
        && result.is_none()
    {
        result = f().map_err(Error::Exec)?;
    }

    result.ok_or(Error::Timeout(timeout))
}

/// Invokes a function repeatedly until it yields true; if a timeout is
/// reached, the function will panic
pub fn loop_timeout_panic<T>(
    timeout: Duration,
    mut f: impl FnMut() -> Option<T>,
) -> T {
    loop_timeout(timeout, || Ok(f())).unwrap()
}
