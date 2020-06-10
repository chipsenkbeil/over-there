mod cli_common;

use cli_common as cc;
use std::time::Duration;

#[tokio::test]
#[ignore]
async fn test_tcp_client_server_multiple_msgs() {
    let addr_str = cc::setup();
    let output_file_path = tempfile::NamedTempFile::new()
        .expect("Failed to create temporary file")
        .path()
        .to_string_lossy()
        .to_string();

    tokio::select! {
        // Server execution
        _ = cc::run(cc::build_server_opts(
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
            cc::run(cc::build_client_opts(
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
            cc::run(cc::build_client_opts(
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
