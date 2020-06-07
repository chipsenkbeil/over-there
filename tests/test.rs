use clap::derive::Clap;
use std::time::Duration;

fn setup() -> String {
    env_logger::init();
    String::from("127.0.0.1:60123")
}

async fn run(args: Vec<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let opts = over_there::Opts::parse_from(args);
    over_there::run(opts).await
}

fn build_server_opts<'a>(
    addr: &'a str,
    transport: &'a str,
    mut other_opts: Vec<&'a str>,
) -> Vec<&'a str> {
    let mut opts = vec!["over-there", "server", addr, "-t", transport];
    opts.append(&mut other_opts);
    opts
}

fn build_client_opts<'a>(
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

#[tokio::test]
async fn test_tcp_client_server_multiple_msgs() {
    let addr_str = setup();
    let output_file_path = tempfile::NamedTempFile::new()
        .expect("Failed to create temporary file")
        .path()
        .to_string_lossy()
        .to_string();

    tokio::select! {
        // Server execution
        _ = run(build_server_opts(
                addr_str.as_str(),
                "tcp",
                vec![],
            )) => {
            panic!("Server exited before client requests finished!");
        },

        // Client requests
        _ = async {
            // Wait a little bit for server to start
            tokio::time::delay_for(Duration::from_millis(100)).await;

            // Communicate to server using client
            run(build_client_opts(
                addr_str.as_str(),
                "tcp",
                vec!["exec", "echo", "test"],
                &output_file_path,
            )).await.expect("Failed to send first request");

            // Wait for request to be processed
            tokio::time::delay_for(Duration::from_millis(100)).await;

            // Check the file to see that we got proper output
            let output = tokio::fs::read_to_string(&output_file_path)
                .await
                .expect("Failed to read output file");
            assert_eq!(output, "test\n");

            // Clear the output file
            tokio::fs::remove_file(&output_file_path)
                .await
                .expect("Failed to clear file");

            // On TCP issue, this would stall
            run(build_client_opts(
                addr_str.as_str(),
                "tcp",
                vec!["exec", "echo", "test2"],
                &output_file_path,
            )).await.expect("Failed to send second request");

            // Wait for request to be processed
            tokio::time::delay_for(Duration::from_millis(100)).await;

            // Check the file to see that we got proper output
            let output = tokio::fs::read_to_string(&output_file_path)
                .await
                .expect("Failed to read output file");
            assert_eq!(output, "test2\n");

            // Clear the output file
            tokio::fs::remove_file(&output_file_path)
                .await
                .expect("Failed to clear file");
        } => {},
    }
}
