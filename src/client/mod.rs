use clap::Clap;
use over_there_core::Client;
use std::error::Error;
use std::time::Duration;

#[derive(Clap, Debug)]
pub struct ClientCommand {
    #[clap(long, parse(try_from_str = parse_duration), default_value = "5")]
    /// Timeout (in seconds) used when communicating with the server
    pub timeout: Duration,
}

pub fn hello() {
    println!("HELLO! {:?}", Client::DEFAULT_TIMEOUT);
}

fn parse_duration(s: &str) -> Result<Duration, Box<dyn Error>> {
    let secs: f64 = s.parse()?;
    Ok(Duration::from_secs_f64(secs))
}
