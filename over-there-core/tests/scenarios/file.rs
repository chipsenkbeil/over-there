use over_there_core::Client;
use std::time::Duration;

pub async fn test(mut client: Client) {
    // Ensure that we fail after 2.5s
    client.timeout = Duration::from_millis(2500);

    // Produce a new directory to work in
    // let dir = tempfile::TempDir::new().unwrap();
    // let dir_path = dir.path().to_string_lossy().to_string();
    let dir = std::env::temp_dir();
    let dir_path = dir.to_string_lossy().to_string();

    // Make a new file that we'll edit, read, and validate
    // let file_path = tempfile::NamedTempFile::new_in(&dir)
    //     .unwrap()
    //     .into_temp_path()
    //     .to_string_lossy()
    //     .to_string();
    let mut file_path = dir.clone();
    file_path.push("test_file.txt");
    let file_path = file_path.to_string_lossy().to_string();

    // let dir_contents = client.ask_list_dir_contents(&dir_path).await.unwrap();
    // assert_eq!(dir_contents.len(), 0);

    let mut file = client.ask_open_file(&file_path).await.unwrap();
    client
        .ask_write_file(&mut file, b"Hello!\nThis is a test!\nGoodbye!")
        .await
        .unwrap();

    // let dir_contents = client.ask_list_dir_contents(&dir_path).await.unwrap();
    // assert_eq!(dir_contents.len(), 1);

    let result =
        String::from(std::str::from_utf8(&client.ask_read_file(&file).await.unwrap()).unwrap());
    assert_eq!(result, "Hello!\nThis is a test!\nGoodbye!");
}
