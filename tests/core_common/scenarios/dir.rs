use over_there::core::ConnectedClient;

pub async fn async_test(mut client: ConnectedClient) {
    // Produce a new directory to work in
    let dir = tempfile::TempDir::new().unwrap();
    let root = dir.as_ref().join("test").join("dir");
    let root_str = root.as_path().to_string_lossy().to_string();
    let root_str_2 = format!("{}-2", root_str.clone());

    // Create a new dir where we'll play around
    client
        .ask_create_dir(root_str.clone(), true)
        .await
        .expect("Failed to create dir");

    let file = client
        .ask_open_file(
            root.as_path()
                .join("test-file")
                .to_string_lossy()
                .to_string(),
        )
        .await
        .expect("Failed to create file")
        .into();

    let dir_contents = client
        .ask_list_dir_contents(root_str.clone())
        .await
        .expect("Failed to get dir contents")
        .entries;
    assert_eq!(dir_contents.len(), 1);

    // Moving the directory should fail
    if client
        .ask_rename_dir(root_str.clone(), root_str_2.clone())
        .await
        .is_ok()
    {
        panic!("Succeeded in renaming a dir with open file");
    }

    // Close our file and try again
    client
        .ask_close_file(&file)
        .await
        .expect("Failed to close file");
    client
        .ask_rename_dir(root_str.clone(), root_str_2.clone())
        .await
        .expect("Failed to rename dir");

    // Verfy failure to list contents for old path
    if client.ask_list_dir_contents(root_str.clone()).await.is_ok() {
        panic!("Succeeded in listing contents for old dir path");
    }

    // List contents of renamed dir
    let dir_contents = client
        .ask_list_dir_contents(root_str_2.clone())
        .await
        .expect("Failed to get renamed dir contents")
        .entries;
    assert_eq!(dir_contents.len(), 1);

    // Finally remove the renamed dir
    client
        .ask_remove_dir(root_str_2.clone(), true)
        .await
        .expect("Failed to remove renamed dir");

    // Verfy failure to list contents for old path
    if client
        .ask_list_dir_contents(root_str_2.clone())
        .await
        .is_ok()
    {
        panic!("Succeeded in listing contents for dir after removed");
    }
}
