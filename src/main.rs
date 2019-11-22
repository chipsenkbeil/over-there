use clap::{crate_authors, crate_description, crate_version, App, Arg, SubCommand};

use ring::rand::SecureRandom;
use ring::{digest, hmac, rand};

fn main() {
    let matches = App::new("Over There")
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())
        .subcommand(SubCommand::with_name("daemon").about("Launch daemon instance"))
        .subcommand(SubCommand::with_name("client").about("Issue commands to daemon instance"))
        .get_matches();

    match matches.subcommand_name() {
        Some("daemon") => println!("Running daemon!"),
        Some("client") => println!("Running client!"),
        _ => println!("Print help!"),
    }

    println!("Hello, world!");
}
