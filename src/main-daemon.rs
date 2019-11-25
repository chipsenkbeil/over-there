use clap::{crate_authors, crate_description, crate_version, App, Arg, SubCommand};

use ring::rand::SecureRandom;
use ring::{digest, hmac, rand};

use over_there;

fn main() {
    let matches = App::new("Over There Daemon")
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())
        .get_matches();

    let x = Some(3);
    let x = x.map(|a| a + 3);

    println!("Hello, world! {}", x.unwrap());
    over_there::run_me();
}
