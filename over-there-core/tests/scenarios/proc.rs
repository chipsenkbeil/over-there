use over_there_core::{Client, RemoteProc};
use std::time::{Duration, Instant};

const OUTPUT_TIMEOUT: Duration = Duration::from_millis(2500);

pub async fn async_test(client: Client) {
    // Perform an echo, which will run once to completion
    let proc = client
        .ask_exec_proc(String::from("echo"), vec![String::from("hello")])
        .await
        .unwrap();
    let output = wait_for_nonempty_output(&client, &proc, OUTPUT_TIMEOUT).await;
    assert_eq!(output, "hello\n");

    // Start a cat proc where we can feed in data and get it back out
    let proc = client
        .ask_exec_proc(String::from("cat"), vec![])
        .await
        .unwrap();
    client.ask_write_stdin(&proc, b"test\n").await.unwrap();

    let output = wait_for_nonempty_output(&client, &proc, OUTPUT_TIMEOUT).await;
    assert_eq!(output, "test\n");

    // Write again to proc to prove that it hasn't closed input
    client
        .ask_write_stdin(&proc, b"another test\n")
        .await
        .unwrap();

    let output = wait_for_nonempty_output(&client, &proc, OUTPUT_TIMEOUT).await;
    assert_eq!(output, "another test\n");
}

async fn wait_for_nonempty_output(client: &Client, proc: &RemoteProc, timeout: Duration) -> String {
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
