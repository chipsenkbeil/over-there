use over_there_core::Client;

pub async fn async_test(client: Client) {
    // Perform an echo, which will run once to completion
    let proc = client.ask_exec_proc("echo", vec!["hello"]).await.unwrap();
    let output = client.ask_get_stdout(&proc).await.unwrap();
    let output = String::from_utf8_lossy(&output);
    assert_eq!(output, "hello\n");

    // Start a cat proc where we can feed in data and get it back out
    let proc = client.ask_exec_proc("cat", vec![]).await.unwrap();
    client.ask_write_stdin(&proc, b"test\n").await.unwrap();

    let output = client.ask_get_stdout(&proc).await.unwrap();
    let output = String::from_utf8_lossy(&output);
    assert_eq!(output, "test\n");

    // Write again to proc to prove that it hasn't closed input
    client
        .ask_write_stdin(&proc, b"another test\n")
        .await
        .unwrap();

    let output = client.ask_get_stdout(&proc).await.unwrap();
    let output = String::from_utf8_lossy(&output);
    assert_eq!(output, "another test\n");
}
