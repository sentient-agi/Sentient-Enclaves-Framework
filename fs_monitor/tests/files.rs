use std::fs::File;
use std::io::Write;
use std::time::Duration;
use uuid::Uuid;
use std::path::Path;

mod common;
use common::{setup_test_environment, wait_for_file_state};
use fs_monitor::monitor_module::{state::FileState, fs_utils::handle_path};
use fs_monitor::hash::hasher::{hash_file, bytes_to_hex};
use fs_monitor::hash::retrieve_hash;


#[tokio::test]
async fn file_modification_simple() -> Result<(), Box<dyn std::error::Error>>{
    let setup = setup_test_environment().await?;

    // Create a temp file
    let file_path = format!("test_{}.txt", Uuid::new_v4());
    let mut file = File::create(file_path.clone())?;
    let file_path = handle_path(&file_path);
    eprintln!("Path: {}", file_path);
    
    assert!(wait_for_file_state(&setup.file_infos, &file_path, Some(FileState::Created), 4000).await);

    // Write random data to the file
    let _ = file.write_all(b"SOME DATA");
    let _ = file.flush();

    file.write_all(b" MORE DATA").unwrap();
    let _ = file.flush();
    assert!(wait_for_file_state(&setup.file_infos, &file_path, Some(FileState::Modified), 4000).await);

    // Close the file
    file.flush().unwrap();
    file.sync_all().unwrap(); 
    drop(file);
    
    assert!(wait_for_file_state(&setup.file_infos, &file_path, Some(FileState::Closed), 4000).await);
    
    // Calculate hash
    let calculated_hash = bytes_to_hex(&hash_file(&file_path).unwrap());
    // Check hashes match
    let retrieved_hash = retrieve_hash(&file_path, &setup.file_infos, &setup.hash_infos).await.unwrap();
    eprintln!("Calculated Hash: {}, Retrieved Hash: {}", calculated_hash, retrieved_hash);
    assert_eq!(calculated_hash, retrieved_hash);
    
    // Cleanup
    eprintln!("Attempting to delete file: {}", file_path);
    std::fs::remove_file(file_path).ok();
    setup.cleanup().await?;

    eprintln!("=== Test complete, forcing exit ===");
    Ok(())
}

#[tokio::test]
async fn file_deletion_simple() -> Result<(), Box<dyn std::error::Error>> {
    let setup = setup_test_environment().await?;

    let file_path = format!("test_{}.txt", Uuid::new_v4());
    let mut file = File::create(file_path.clone())?;
    let file_path = handle_path(&file_path);
    eprintln!("Path: {}", file_path);
    assert!(wait_for_file_state(&setup.file_infos, &file_path, Some(FileState::Created), 4000).await);

    let _ = file.write_all(b"SOME DATA");
    let _ = file.flush();
    assert!(wait_for_file_state(&setup.file_infos, &file_path, Some(FileState::Modified), 4000).await);

    // Close the file
    file.flush().unwrap();
    file.sync_all().unwrap(); 
    drop(file);

    assert!(wait_for_file_state(&setup.file_infos, &file_path, Some(FileState::Closed), 4000).await);

    eprintln!("Attempting to delete file: {}", file_path);
    std::fs::remove_file(file_path.clone()).ok();

    assert!(wait_for_file_state(&setup.file_infos, &file_path, None, 4000).await);

    // Cleanup KV store
    setup.cleanup().await?;
    eprintln!("=== Test complete, forcing exit ===");
    Ok(())
}

#[tokio::test]
async fn file_deletion_empty() -> Result<(), Box<dyn std::error::Error>> {
    let setup = setup_test_environment().await?;

    let file_path = format!("test_{}.txt", Uuid::new_v4());
    let _file = File::create(file_path.clone())?;
    let file_path = handle_path(&file_path);
    eprintln!("Path: {}", file_path);
    assert!(wait_for_file_state(&setup.file_infos, &file_path, Some(FileState::Created), 4000).await);

    eprintln!("Attempting to delete file: {}", file_path);
    std::fs::remove_file(file_path.clone()).ok();
    assert!(wait_for_file_state(&setup.file_infos, &file_path, None, 4000).await);
    
    setup.cleanup().await?;

    eprintln!("=== Test complete, forcing exit ===");
    Ok(())
}

#[tokio::test]
async fn file_rename_basic() -> Result<(), Box<dyn std::error::Error>> {
    let setup = setup_test_environment().await?;

    let file_path = format!("test_{}.txt", Uuid::new_v4());
    let mut file = File::create(file_path.clone())?;
    let file_path = handle_path(&file_path);
    eprintln!("Path: {}", file_path);
    assert!(wait_for_file_state(&setup.file_infos, &file_path, Some(FileState::Created), 4000).await);

    // Perform dummy writes
    let _ = file.write_all(b"SOME DATA");
    let _ = file.flush();
    assert!(wait_for_file_state(&setup.file_infos, &file_path, Some(FileState::Modified), 4000).await);

    // Close the file
    file.flush().unwrap();
    file.sync_all().unwrap(); 
    drop(file);

    tokio::time::sleep(Duration::from_secs(2)).await;
    let old_hash = retrieve_hash(&file_path, &setup.file_infos, &setup.hash_infos).await.unwrap();
    
    eprintln!("Attempting to Rename file: {}", file_path);
    let new_file_path = format!("test_{}.txt", Uuid::new_v4());
    std::fs::rename(file_path.clone(), new_file_path.clone())?;
    
    let new_file_path = handle_path(&new_file_path);
    assert!(wait_for_file_state(&setup.file_infos, &file_path, None, 4000).await);
    assert!(wait_for_file_state(&setup.file_infos, &new_file_path, Some(FileState::Closed), 4000).await);
    let new_hash = retrieve_hash(&new_file_path, &setup.file_infos, &setup.hash_infos).await.unwrap();

    eprintln!("Old Hash: {}, New Hash: {}", old_hash, new_hash);
    assert_eq!(old_hash, new_hash);

    std::fs::remove_file(new_file_path)?;
    setup.cleanup().await?;
    Ok(())
}

#[tokio::test]
async fn file_rename_to_unwatched() -> Result<(), Box<dyn std::error::Error>> {
    let setup = setup_test_environment().await?;

    let file_path = format!("test_{}.txt", Uuid::new_v4());
    let mut file = File::create(file_path.clone())?;
    let file_path = handle_path(&file_path);
    eprintln!("Path: {}", file_path);
    assert!(wait_for_file_state(&setup.file_infos, &file_path, Some(FileState::Created), 4000).await);

    // Perform dummy writes
    let _ = file.write_all(b"SOME DATA");
    let _ = file.flush();
    assert!(wait_for_file_state(&setup.file_infos, &file_path, Some(FileState::Modified), 4000).await);

    // Close the file
    file.flush().unwrap();
    file.sync_all().unwrap(); 
    drop(file);
    
    assert!(wait_for_file_state(&setup.file_infos, &file_path, Some(FileState::Closed), 4000).await);

    // Move the file out of this directory.
    let new_path = Path::new("..").join(&file_path);
    std::fs::rename(file_path.clone(), new_path.clone())?;
    assert!(wait_for_file_state(&setup.file_infos, &file_path, None, 4000).await);

    // Cleanup the renamed file
    std::fs::remove_file(new_path)?;
    setup.cleanup().await?;
    Ok(())
}

#[tokio::test]
async fn file_rename_to_watched() -> Result<(), Box<dyn std::error::Error>> {
    let setup = setup_test_environment().await?;

    // Create a temporary directory outside the watched path
    let temp_dir = std::env::temp_dir().join(format!("test_dir_{}", Uuid::new_v4()));
    std::fs::create_dir_all(&temp_dir)?;

    // Create a file in the temporary directory
    let file_name = format!("test_{}.txt", Uuid::new_v4());
    let source_path = temp_dir.join(&file_name);
    let mut file = File::create(&source_path)?;
    file.write_all(b"TEST DATA")?;
    file.flush()?;
    drop(file);

    // Move the file into the watched directory
    let target_path = Path::new(".").join(&file_name);
    std::fs::rename(&source_path, &target_path)?;
    let target_path = handle_path(target_path.to_str().unwrap());

    // Verify the file is detected and processed
    assert!(wait_for_file_state(&setup.file_infos, &target_path, Some(FileState::Closed), 4000).await);

    // Verify that hash calculation is performed and is correct
    let calculated_hash = bytes_to_hex(&hash_file(&target_path).unwrap());
    let retrieved_hash = retrieve_hash(&target_path, &setup.file_infos, &setup.hash_infos).await.unwrap();
    eprintln!("Calculated Hash: {}, Retireved Hash: {}", calculated_hash, retrieved_hash);
    assert_eq!(calculated_hash, retrieved_hash);

    // Clean up
    std::fs::remove_file(target_path)?;
    std::fs::remove_dir_all(temp_dir)?;
    setup.cleanup().await?;
    Ok(())
}

#[tokio::test]
async fn file_rename_to_ignored() -> Result<(), Box<dyn std::error::Error>>{
    let setup = setup_test_environment().await?;

    let file_path = format!("test_{}.txt", Uuid::new_v4());
    let mut file = File::create(file_path.clone())?;
    let file_path = handle_path(&file_path);
    eprintln!("Path: {}", file_path);
    assert!(wait_for_file_state(&setup.file_infos, &file_path, Some(FileState::Created), 4000).await);

    // Perform dummy writes
    let _ = file.write_all(b"SOME DATA");
    let _ = file.flush();
    assert!(wait_for_file_state(&setup.file_infos, &file_path, Some(FileState::Modified), 4000).await);

    // Close the file
    file.flush().unwrap();
    file.sync_all().unwrap(); 
    drop(file);

    tokio::time::sleep(Duration::from_secs(2)).await;
    
    // Rename to an ignored path
    eprintln!("Attempting to Rename file to an ignored path: {}", file_path);
    let new_file_path = format!("tmp_test_{}.txt", Uuid::new_v4());
    std::fs::rename(file_path.clone(), new_file_path.clone())?;
    
    let new_file_path = handle_path(&new_file_path);
    assert!(wait_for_file_state(&setup.file_infos, &file_path, None, 4000).await);
    assert!(wait_for_file_state(&setup.file_infos, &new_file_path, None, 4000).await);

    std::fs::remove_file(new_file_path)?;
    setup.cleanup().await?;
    Ok(())
}

#[tokio::test]
async fn file_rename_from_ignored() -> Result<(), Box<dyn std::error::Error>>{
    let setup = setup_test_environment().await?;

    let file_path = format!("tmp_test_{}.txt", Uuid::new_v4());
    let mut file = File::create(file_path.clone())?;
    let file_path = handle_path(&file_path);
    eprintln!("Path: {}", file_path);
    assert!(wait_for_file_state(&setup.file_infos, &file_path, None, 4000).await);

    let new_file_path = format!("test_{}.txt", Uuid::new_v4());
    eprintln!("Attempting to Rename file from an ignored path: {} to: {}", file_path, new_file_path);

    let new_file_path = handle_path(&new_file_path);
    std::fs::rename(file_path.clone(), new_file_path.clone())?;
    assert!(wait_for_file_state(&setup.file_infos, &new_file_path, Some(FileState::Created), 4000).await);

    // Perform dummy write
    let _ = file.write_all(b"SOME DATA");
    let _ = file.flush();
    assert!(wait_for_file_state(&setup.file_infos, &new_file_path, Some(FileState::Modified), 4000).await);

    // Close the file
    file.flush().unwrap();
    file.sync_all().unwrap(); 
    drop(file);

    tokio::time::sleep(Duration::from_secs(2)).await;
    assert!(wait_for_file_state(&setup.file_infos, &new_file_path, Some(FileState::Closed), 4000).await);

    std::fs::remove_file(new_file_path)?;
    setup.cleanup().await?;
    Ok(())
}