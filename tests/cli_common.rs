pub fn setup() -> String {
    env_logger::init();
    String::from("127.0.0.1:60123")
}

pub async fn run(args: Vec<&str>) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(feature = "cli")]
    use clap::derive::Clap;

    #[cfg(feature = "cli")]
    let opts = over_there::cli::Opts::parse_from(args);

    #[cfg(feature = "cli")]
    return over_there::cli::run(opts).await;

    #[cfg(not(feature = "cli"))]
    return Err("cli feature not enabled".into());
}

pub fn build_server_opts<'a>(
    addr: &'a str,
    transport: &'a str,
    mut other_opts: Vec<&'a str>,
) -> Vec<&'a str> {
    let mut opts = vec!["over-there", "server", addr, "-t", transport];
    opts.append(&mut other_opts);
    opts
}

pub fn build_client_opts<'a>(
    addr: &'a str,
    transport: &'a str,
    mut other_opts: Vec<&'a str>,
    output_path: &'a str,
) -> Vec<&'a str> {
    let mut opts = vec![
        "over-there",
        "client",
        addr,
        "-t",
        transport,
        "--redirect-stdout",
        output_path,
    ];
    opts.append(&mut other_opts);
    opts
}
