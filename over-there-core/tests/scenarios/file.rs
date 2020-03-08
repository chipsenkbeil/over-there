use over_there_core::Client;

pub async fn async_test(mut client: Client) {
    // Produce a new directory to work in
    let dir = tempfile::TempDir::new().unwrap();
    let dir_path = dir.path().to_string_lossy().to_string();

    // Make a new file that we'll edit, read, and validate
    let file_path = tempfile::NamedTempFile::new_in(&dir)
        .unwrap()
        .into_temp_path()
        .to_string_lossy()
        .to_string();

    let dir_contents = client
        .ask_list_dir_contents(dir_path.clone())
        .await
        .expect("Failed to get empty dir contents");
    assert_eq!(dir_contents.len(), 0);

    // Open/create file with read & write access
    let mut file = client
        .ask_open_file(file_path.clone())
        .await
        .expect("Failed to open file");
    client
        .ask_write_file(&mut file, b"Hello!\nThis is a test!\nGoodbye!")
        .await
        .expect("Failed to write to file");

    let dir_contents = client
        .ask_list_dir_contents(dir_path.clone())
        .await
        .expect("Failed to get dir contents");
    assert_eq!(dir_contents.len(), 1);

    // Close the file, rename it, re-open it, read content
    client
        .ask_close_file(&file)
        .await
        .expect("Failed to close file");

    let file_path_2 = format!("{}.2", file_path.clone());
    client
        .ask_rename_file(file_path, file_path_2.clone())
        .await
        .expect("Failed to rename file");

    let file = client
        .ask_open_file(file_path_2.clone())
        .await
        .expect("Failed to open renamed file");

    let result = String::from(
        std::str::from_utf8(&client.ask_read_file(&file).await.unwrap())
            .expect("Failed to read renamed file"),
    );
    assert_eq!(result, "Hello!\nThis is a test!\nGoodbye!");

    // Close file and remove it
    client
        .ask_close_file(&file)
        .await
        .expect("Failed to close file");
    client
        .ask_remove_file(file_path_2)
        .await
        .expect("Failed to remove renamed file");

    // Verify it's gone
    let dir_contents = client
        .ask_list_dir_contents(dir_path)
        .await
        .expect("Failed to get dir contents");
    assert_eq!(dir_contents.len(), 0);
}
