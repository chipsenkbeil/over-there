use std::time::{Duration, SystemTime, SystemTimeError};

#[derive(Debug)]
pub enum Error {
    Exec(Box<dyn std::error::Error>),
    SystemTime(SystemTimeError),
    Timeout(Duration),
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match &*self {
            Error::Exec(e) => write!(f, "Execution Error: {:?}", e),
            Error::SystemTime(e) => write!(f, "SystemTime Error: {:?}", e),
            Error::Timeout(d) => write!(f, "Exceeded Duration: {:?}", d),
        }
    }
}

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

pub fn loop_timeout_panic(timeout: Duration, mut f: impl FnMut() -> bool) -> () {
    loop_timeout(timeout, || Ok(f())).unwrap()
}
