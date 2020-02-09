use clap::Clap;
use std::error::Error;
use std::time::Duration;

#[derive(Clap, Debug)]
pub struct ServerCommand {
    #[clap(long)]
    pub name: String,

    #[clap(long, parse(try_from_str = parse_duration), default_value = "5")]
    /// Timeout (in seconds) used when communicating with clients and other servers
    pub timeout: Duration,
}

fn parse_duration(s: &str) -> Result<Duration, Box<dyn Error>> {
    let secs: f64 = s.parse()?;
    Ok(Duration::from_secs_f64(secs))
}
