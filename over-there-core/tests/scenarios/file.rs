use over_there_core::Client;

pub async fn async_test(client: Client) {
    // Produce a new directory to work in
    let dir = tempfile::TempDir::new().unwrap();
    let dir_path = dir.path().to_string_lossy().to_string();

    // Make a new file that we'll edit, read, and validate
    let file_path = tempfile::NamedTempFile::new_in(&dir)
        .unwrap()
        .into_temp_path()
        .to_string_lossy()
        .to_string();

    let dir_contents = client.ask_list_dir_contents(&dir_path).await.unwrap();
    assert_eq!(dir_contents.len(), 0);

    // Open/create file with read & write access
    let mut file = client.ask_open_file(&file_path).await.unwrap();
    client
        .ask_write_file(&mut file, b"Hello!\nThis is a test!\nGoodbye!")
        .await
        .unwrap();

    let dir_contents = client.ask_list_dir_contents(&dir_path).await.unwrap();
    assert_eq!(dir_contents.len(), 1);

    let result =
        String::from(std::str::from_utf8(&client.ask_read_file(&file).await.unwrap()).unwrap());
    assert_eq!(result, "Hello!\nThis is a test!\nGoodbye!");
}
