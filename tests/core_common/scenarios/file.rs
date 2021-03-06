use over_there::core::ConnectedClient;

pub async fn async_test(mut client: ConnectedClient) {
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
        .expect("Failed to get empty dir contents")
        .entries;
    assert_eq!(dir_contents.len(), 0);

    // Open/create file with read & write access
    let mut file = client
        .ask_open_file(file_path.clone())
        .await
        .expect("Failed to open file")
        .into();
    client
        .ask_write_file(&mut file, b"Hello!\nThis is a test!\nGoodbye!")
        .await
        .expect("Failed to write to file");

    let dir_contents = client
        .ask_list_dir_contents(dir_path.clone())
        .await
        .expect("Failed to get dir contents")
        .entries;
    assert_eq!(dir_contents.len(), 1);

    let file_path_2 = format!("{}.2", file_path.clone());
    client
        .ask_rename_file(&mut file, file_path_2.clone())
        .await
        .expect("Failed to rename file");

    let mut file = client
        .ask_open_file(file_path_2.clone())
        .await
        .expect("Failed to open renamed file")
        .into();

    let result = String::from(
        std::str::from_utf8(
            &client
                .ask_read_file(&file)
                .await
                .expect("Failed to read file")
                .contents,
        )
        .expect("Failed to read renamed file"),
    );
    assert_eq!(result, "Hello!\nThis is a test!\nGoodbye!");

    client
        .ask_remove_file(&mut file)
        .await
        .expect("Failed to remove renamed file");

    let dir_contents = client
        .ask_list_dir_contents(dir_path)
        .await
        .expect("Failed to get dir contents")
        .entries;
    assert_eq!(dir_contents.len(), 0);
}
