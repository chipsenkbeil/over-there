use over_there_derive::Error;
use std::time::{Duration, SystemTime, SystemTimeError};

#[derive(Debug, Error)]
pub enum Error {
    Exec(Box<dyn std::error::Error>),
    SystemTime(SystemTimeError),
    Timeout(Duration),
}

/// Invokes a function repeatedly until it yields true; if a timeout is reached,
/// the function will return an error
pub fn loop_timeout(
    timeout: Duration,
    mut f: impl FnMut() -> Result<bool, Box<dyn std::error::Error>>,
) -> Result<(), Error> {
    let start_time = SystemTime::now();
    let mut result = false;
    while SystemTime::now()
        .duration_since(start_time)
        .map_err(Error::SystemTime)?
        < timeout
        && !result
    {
        result = f().map_err(Error::Exec)?;
    }

    if result {
        Ok(())
    } else {
        Err(Error::Timeout(timeout))
    }
}

/// Invokes a function repeatedly until it yields true; if a timeout is
/// reached, the function will panic
pub fn loop_timeout_panic(timeout: Duration, mut f: impl FnMut() -> bool) {
    loop_timeout(timeout, || Ok(f())).unwrap()
}
