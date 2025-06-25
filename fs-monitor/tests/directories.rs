use uuid::Uuid;
use std::path::Path;
mod common;
use common::{setup_test_environment, wait_for_file_state, create_test_directory_hierarchy};
use fs_monitor::monitor_module::{state::FileState, fs_utils::handle_path};
use fs_monitor::hash::retrieve_hash;

#[tokio::test]
async fn directory_deletion() -> Result<(), Box<dyn std::error::Error>> {
    let setup = setup_test_environment().await?;

    // Create hierarchical directory
    let (root_dir, file_paths) = create_test_directory_hierarchy(Path::new("."))?;

    // Wait for all files to be detected
    for file_path in &file_paths {
        let normalized_path = handle_path(file_path);
        println!("Normalized path: {}", normalized_path);
        assert!(wait_for_file_state(&setup.file_infos, &normalized_path, Some(FileState::Closed), 4000).await);
    }

    // Delete the directory
    std::fs::remove_dir_all(&root_dir)?;

    // Verify files are removed from tracking
    for file_path in &file_paths {
        let normalized_path = handle_path(file_path);
        assert!(wait_for_file_state(&setup.file_infos, &normalized_path, None, 4000).await);
    }

    // Cleanup KV store
    setup.cleanup().await?;
    Ok(())
}

#[tokio::test]
async fn directory_rename_simple() -> Result<(), Box<dyn std::error::Error>> {
    let setup = setup_test_environment().await?;

    // Create hierarchical directory
    let (root_dir, file_paths) = create_test_directory_hierarchy(Path::new("."))?;

    // Wait for all files to be detected
    let mut file_hashes = Vec::new();
    for file_path in &file_paths {
        let normalized_path = handle_path(file_path);
        assert!(wait_for_file_state(&setup.file_infos, &normalized_path, Some(FileState::Closed), 4000).await);

        // Store hash for later comparison
        let hash = retrieve_hash(&normalized_path, &setup.file_infos, &setup.hash_infos).await.unwrap();
        file_hashes.push((normalized_path, hash));
    }

    // Rename the directory
    let new_dir_name = format!("renamed_dir_{}", Uuid::new_v4());
    let new_dir_path = Path::new(&new_dir_name);
    std::fs::rename(&root_dir, new_dir_path)?;

    // Verify old paths are gone
    for (path, _) in &file_hashes {
        assert!(wait_for_file_state(&setup.file_infos, path, None, 4000).await);
    }

    // Verify new paths exist with same hashes
    for (i, file_path) in file_paths.iter().enumerate() {
        let new_path = file_path.replace(&root_dir, &new_dir_name);
        let normalized_new_path = handle_path(&new_path);

        // Wait for file to be detected at new location
        assert!(wait_for_file_state(&setup.file_infos, &normalized_new_path, Some(FileState::Closed), 4000).await);

        // Check hash is the same
        let new_hash = retrieve_hash(&normalized_new_path, &setup.file_infos, &setup.hash_infos).await.unwrap();
        assert_eq!(file_hashes[i].1, new_hash);
    }

    // Clean up
    std::fs::remove_dir_all(new_dir_path)?;
    setup.cleanup().await?;

    Ok(())
}

#[tokio::test]
async fn directory_rename_to_watched() -> Result<(), Box<dyn std::error::Error>> {
    let setup = setup_test_environment().await?;

    // Create directory in unwatched location
    let (root_dir, file_paths) = create_test_directory_hierarchy(std::env::temp_dir().as_path())?;

    // Move directory to watched location
    let target_dir = format!("moved_dir_{}", Uuid::new_v4());
    std::fs::rename(&root_dir, &target_dir)?;

    // Verify files are detected in new location
    for file_path in &file_paths {
        let new_path = file_path.replace(&root_dir, &target_dir);
        let normalized_new_path = handle_path(&new_path);

        assert!(wait_for_file_state(&setup.file_infos, &normalized_new_path, Some(FileState::Closed), 4000).await);
    }

    // Clean up
    std::fs::remove_dir_all(&target_dir)?;
    setup.cleanup().await?;

    Ok(())
}

#[tokio::test]
async fn directory_rename_to_unwatched() -> Result<(), Box<dyn std::error::Error>> {
    let setup = setup_test_environment().await?;

    // Create directory in watched location
    let (root_dir, file_paths) = create_test_directory_hierarchy(Path::new("."))?;

    // Wait for all files to be detected
    for file_path in &file_paths {
        let normalized_path = handle_path(file_path);
        assert!(wait_for_file_state(&setup.file_infos, &normalized_path, Some(FileState::Closed), 4000).await);
    }

    // Move directory to unwatched location
    let target_dir = std::env::temp_dir().join(format!("moved_unwatched_{}", Uuid::new_v4()));
    std::fs::rename(&root_dir, &target_dir)?;

    // Verify files are no longer tracked
    for file_path in &file_paths {
        let normalized_path = handle_path(file_path);
        assert!(wait_for_file_state(&setup.file_infos, &normalized_path, None, 4000).await);
    }

    // Clean up
    std::fs::remove_dir_all(target_dir)?;
    setup.cleanup().await?;

    Ok(())
}

#[tokio::test]
async fn directory_rename_to_ignored() -> Result<(), Box<dyn std::error::Error>> {
    let setup = setup_test_environment().await?;

    // Create directory in watched location
    let (root_dir, file_paths) = create_test_directory_hierarchy(Path::new("."))?;

    // Wait for all files to be detected
    for file_path in &file_paths {
        let normalized_path = handle_path(file_path);
        assert!(wait_for_file_state(&setup.file_infos, &normalized_path, Some(FileState::Closed), 4000).await);
    }

    // Rename directory to an ignored path (using tmp_* pattern from fs_ignore)
    let new_dir_name = format!("tmp_{}", Uuid::new_v4());
    let new_dir_path = Path::new(&new_dir_name);
    std::fs::rename(&root_dir, new_dir_path)?;

    // Verify files are no longer tracked (should be removed because path is now ignored)
    for file_path in &file_paths {
        let normalized_path = handle_path(file_path);
        assert!(wait_for_file_state(&setup.file_infos, &normalized_path, None, 4000).await);
    }

    // Clean up
    std::fs::remove_dir_all(new_dir_path)?;
    setup.cleanup().await?;

    Ok(())
}

#[tokio::test]
async fn directory_rename_from_ignored() -> Result<(), Box<dyn std::error::Error>> {
    let setup = setup_test_environment().await?;

    // Create a directory with an ignored name pattern
    let ignored_dir_name = format!("tmp_{}", Uuid::new_v4());
    let ignored_dir_path = Path::new(&ignored_dir_name);
    std::fs::create_dir_all(ignored_dir_path)?;

    // Create the directory hierarchy inside the ignored directory
    let (_, file_paths) = create_test_directory_hierarchy(ignored_dir_path)?;

    // Verify files in ignored directory are not tracked
    for file_path in &file_paths {
        let normalized_path = handle_path(file_path);
        assert!(wait_for_file_state(&setup.file_infos, &normalized_path, None, 4000).await);
    }

    // Rename the directory to non-ignored name
    let new_dir_name = format!("watched_dir_{}", Uuid::new_v4());
    let new_dir_path = Path::new(&new_dir_name);
    std::fs::rename(ignored_dir_path, new_dir_path)?;

    // Verify files are now tracked after moving to non-ignored directory
    for file_path in &file_paths {
        let new_path = file_path.replace(&ignored_dir_name, &new_dir_name);
        let normalized_new_path = handle_path(&new_path);

        assert!(wait_for_file_state(&setup.file_infos, &normalized_new_path, Some(FileState::Closed), 4000).await);
    }

    // Clean up
    std::fs::remove_dir_all(new_dir_path)?;
    setup.cleanup().await?;

    Ok(())
}

#[tokio::test]
async fn directory_rename_to_dotcache() -> Result<(), Box<dyn std::error::Error>> {
    let setup = setup_test_environment().await?;

    // Create directory in watched location
    let (root_dir, file_paths) = create_test_directory_hierarchy(Path::new("."))?;

    // Wait for all files to be detected
    for file_path in &file_paths {
        let normalized_path = handle_path(file_path);
        assert!(wait_for_file_state(&setup.file_infos, &normalized_path, Some(FileState::Closed), 4000).await);
    }

    let cache_dir = ".cache";
    std::fs::create_dir_all(cache_dir)?; // Ensure .cache exists
    let new_dir_path = Path::new(cache_dir).join(format!("cache_content_{}", Uuid::new_v4()));
    std::fs::rename(&root_dir, &new_dir_path)?;

    // Verify files are no longer tracked
    for file_path in &file_paths {
        let normalized_path = handle_path(file_path);
        assert!(wait_for_file_state(&setup.file_infos, &normalized_path, None, 4000).await);
    }

    // Clean up
    std::fs::remove_dir_all(cache_dir)?;
    setup.cleanup().await?;

    Ok(())
}
