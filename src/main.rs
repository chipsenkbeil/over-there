use over_there::{client, Command, Opts};
use over_there_auth::NoopAuthenticator;
use over_there_core::{Client, Server};
use over_there_crypto::NoopBicrypter;
use over_there_transport::constants;
use over_there_utils::nonblocking;
use std::io;
use tokio::runtime::Runtime;

fn main() {
    let opt = Opts::parse();

    match &opt.command {
        Command::Server(s) => {
            let server = Server::listen_udp(
                s.addr.ip(),
                vec![s.addr.port()],
                constants::DEFAULT_TTL,
                NoopAuthenticator,
                NoopBicrypter,
                |_| true,
            )
            .expect("Failed to start server");

            // Let server run to completion
            server.join().expect("Server concluded with an error");
        }
        Command::Client(c) => {
            let client = Client::connect_udp(
                c.addr,
                constants::DEFAULT_TTL,
                NoopAuthenticator,
                NoopBicrypter,
                |_| true,
            )
            .expect("Failed to connect with client");

            let mut rt = Runtime::new().expect("Failed to start runtime");
            match &c.command {
                client::Subcommand::Version(_) => println!(
                    "{}",
                    rt.block_on(async {
                        client.ask_version().await.expect("Failed to get version")
                    })
                ),
                client::Subcommand::Capabilities(_) => println!(
                    "{:?}",
                    rt.block_on(async {
                        client
                            .ask_capabilities()
                            .await
                            .expect("Failed to get capabilities")
                    })
                ),
                client::Subcommand::RootDir(_) => println!(
                    "{}",
                    rt.block_on(async {
                        client
                            .ask_list_root_dir_contents()
                            .await
                            .expect("Failed to retrieve root directory contents")
                            .iter()
                            .map(|e| {
                                format!(
                                    "[{}{}{}] {}",
                                    if e.is_file { "F" } else { "" },
                                    if e.is_dir { "D" } else { "" },
                                    if e.is_symlink { "S" } else { "" },
                                    e.path,
                                )
                            })
                            .collect::<Vec<String>>()
                            .join("\n")
                    })
                ),
                client::Subcommand::Dir(c) => {
                    println!(
                        "{}",
                        rt.block_on(async {
                            client
                                .ask_list_dir_contents(c.path.clone())
                                .await
                                .expect("Failed to retrieve directory contents")
                                .iter()
                                .map(|e| {
                                    format!(
                                        "[{}{}{}] {}",
                                        if e.is_file { "F" } else { "" },
                                        if e.is_dir { "D" } else { "" },
                                        if e.is_symlink { "S" } else { "" },
                                        e.path,
                                    )
                                })
                                .collect::<Vec<String>>()
                                .join("\n")
                        })
                    );
                }
                client::Subcommand::WriteFile(c) => {
                    println!(
                        "{:?}",
                        rt.block_on(async {
                            let mut file = client
                                .ask_open_file(c.path.clone())
                                .await
                                .expect("Failed to open file");
                            client.ask_write_file(&mut file, c.contents.as_ref()).await
                        })
                    );
                }
                client::Subcommand::ReadFile(c) => {
                    println!(
                        "{}",
                        rt.block_on(async {
                            let file = client
                                .ask_open_file(c.path.clone())
                                .await
                                .expect("Failed to open file");
                            let bytes = client
                                .ask_read_file(&file)
                                .await
                                .expect("Failed to read file");
                            String::from_utf8(bytes)
                                .expect("Failed to translate file back to string")
                        })
                    );
                }
                client::Subcommand::Exec(c) => {
                    rt.block_on(async {
                        let proc = client
                            .ask_exec_proc(c.command.clone(), c.args.clone())
                            .await
                            .expect("Failed to execute proc");

                        // If supporting forwarding stdin, make it nonblocking
                        let stdin = io::stdin();
                        if c.stdin {
                            nonblocking::stdin_set_nonblocking(&stdin)
                                .expect("Unable to make stdin nonblocking");
                        }

                        loop {
                            if c.stdin {
                                use io::BufRead;
                                let mut handle = stdin.lock();
                                let mut line = String::new();
                                match handle.read_line(&mut line) {
                                    Ok(_) => (),
                                    Err(x) if x.kind() == io::ErrorKind::WouldBlock => (),
                                    Err(x) => panic!("Failed to read line of input: {:?}", x),
                                }

                                if !line.is_empty() {
                                    client
                                        .ask_write_stdin(&proc, &line.into_bytes())
                                        .await
                                        .expect("Failed to write stdin");
                                }
                            }

                            let contents = client
                                .ask_get_stdout(&proc)
                                .await
                                .expect("Failed to get stdout");
                            if !contents.is_empty() {
                                println!("{}", String::from_utf8_lossy(&contents));
                            }

                            let contents = client
                                .ask_get_stderr(&proc)
                                .await
                                .expect("Failed to get stderr");
                            if !contents.is_empty() {
                                eprintln!("{}", String::from_utf8_lossy(&contents));
                            }
                        }
                    });
                }
                client::Subcommand::ReattachExec(_c) => unimplemented!(),
            }
        }
    }
}
