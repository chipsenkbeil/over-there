use log::{error, info};
use over_there;
use std::time::Duration;

fn setup() -> String {
    env_logger::init();
    String::from("127.0.0.1:60123")
}

async fn run(args: Vec<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let opts = over_there::Opts::parse_from(args);
    over_there::run(opts).await
}

#[tokio::test]
async fn test_tcp_client_server_multiple_msgs() {
    let addr_str = setup();

    tokio::select! {
        // Server execution
        _ = run(vec!["over-there", "server", addr_str.as_str(), "-t", "Tcp"]) => {
            error!("Server exited before client requests finished!");
        },

        // Client requests
        _ = async {
            // Wait a little bit for server to start
            info!("Waiting for server to start up");
            tokio::time::delay_for(Duration::from_millis(100)).await;

            // Communicate to server using client
            info!("Sending first request");
            run(vec!["over-there", "client", addr_str.as_str(), "-t", "Tcp", "ls-root-dir"])
                .await
                .expect("Failed to send first request");

            // On TCP issue, this would stall - CHIP CHIP CHIP (not stalling in test)
            info!("Sending second request");
            run(vec!["over-there", "client", addr_str.as_str(), "-t", "Tcp", "ls-root-dir"])
                .await
                .expect("Failed to send second request");
        } => {
            info!("All requests have finished!");
        },
    }
}
