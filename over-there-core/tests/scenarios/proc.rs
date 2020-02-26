use over_there_core::{Client, ExecAskError, RemoteProc};
use std::time::{Duration, Instant};

const OUTPUT_TIMEOUT: Duration = Duration::from_millis(2500);

pub async fn async_test(mut client: Client) {
    // Perform an echo, which will run once to completion
    let proc = client
        .ask_exec_proc(String::from("echo"), vec![String::from("hello")])
        .await
        .unwrap();
    let output =
        wait_for_nonempty_output(&mut client, &proc, OUTPUT_TIMEOUT).await;
    assert_eq!(output, "hello\n");

    // Start a cat proc where we can feed in data and get it back out
    let proc = client
        .ask_exec_proc(String::from("cat"), vec![])
        .await
        .unwrap();
    client.ask_write_stdin(&proc, b"test\n").await.unwrap();

    let output =
        wait_for_nonempty_output(&mut client, &proc, OUTPUT_TIMEOUT).await;
    assert_eq!(output, "test\n");

    // Check the status of the proc, which should still be alive
    let status = client.ask_proc_status(&proc).await.unwrap();
    assert_eq!(status.id, proc.id(), "Wrong proc id returned with status");
    assert!(status.is_alive, "Proc reported dead when shouldn't be");
    assert!(
        status.exit_code.is_none(),
        "Got exit code for a running proc"
    );

    // Write again to proc to prove that it hasn't closed input
    client
        .ask_write_stdin(&proc, b"another test\n")
        .await
        .unwrap();

    let output =
        wait_for_nonempty_output(&mut client, &proc, OUTPUT_TIMEOUT).await;
    assert_eq!(output, "another test\n");

    // Kill our proc and verify it's dead
    let status = client.ask_proc_kill(&proc).await.unwrap();
    assert_eq!(status.id, proc.id(), "Wrong proc id returned with status");
    assert!(
        !status.is_alive,
        "Proc reported running when should be dead"
    );

    // Should not be able to get status of proc because it's been removed
    match client.ask_proc_status(&proc).await.unwrap_err() {
        ExecAskError::IoError(x) => {
            assert_eq!(x.kind(), std::io::ErrorKind::InvalidInput)
        }
        x => panic!("Unexpected error: {:?}", x),
    }
}

async fn wait_for_nonempty_output(
    client: &mut Client,
    proc: &RemoteProc,
    timeout: Duration,
) -> String {
    let start = Instant::now();

    while start.elapsed() < timeout {
        let output = client.ask_get_stdout(&proc).await.unwrap();
        let output = String::from_utf8(output).unwrap();
        if !output.is_empty() {
            return output;
        }
    }

    panic!(
        "Reached timeout of {:?} without receiving non-empty output",
        timeout
    );
}
