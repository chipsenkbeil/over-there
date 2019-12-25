use clap::{crate_authors, crate_description, crate_version, App};

fn main() {
    let _matches = App::new("Over There")
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())
        .get_matches();

    let x = Some(3);
    let x = x.map(|a| a + 3);

    println!("Hello, world! {}", x.unwrap());
}
