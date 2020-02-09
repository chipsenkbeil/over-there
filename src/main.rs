use over_there;

fn main() {
    over_there::hello();

    let opt = over_there::Opts::parse();
    println!("{:?}", opt);
}
