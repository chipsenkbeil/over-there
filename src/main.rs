use clap::derive::Clap;
use over_there;
use tokio::runtime::Runtime;

fn main() {
    env_logger::init();
    let opts = over_there::cli::Opts::parse();

    let mut rt = Runtime::new().expect("Failed to start runtime");
    if let Err(x) = rt.block_on(over_there::cli::run(opts)) {
        eprintln!("{}", x);
    }
}
