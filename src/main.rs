use over_there;
use tokio::runtime::Runtime;

fn main() {
    env_logger::init();
    let opts = over_there::Opts::parse();

    let mut rt = Runtime::new().expect("Failed to start runtime");
    rt.block_on(over_there::run(opts)).unwrap();
}
