use std::path::{ Path, PathBuf };
use std::io;
use std::sync::OnceLock;

static WATCH_PATH: OnceLock<PathBuf> = OnceLock::new();

pub fn set_watch_path(path: PathBuf) -> std::io::Result<()> {
    let canonical_path = path.canonicalize()?; // This gives absolute + resolves symlinks
    eprintln!("Setting watch path to {:?}", canonical_path);
    WATCH_PATH.set(canonical_path).map_err(|_| 
        std::io::Error::new(std::io::ErrorKind::AlreadyExists, "WATCH_PATH already initialized")
    )
}

// This normalizes paths returned by notify crate
// for internal use. Make sure this function is called
// always before using path with internal data structures.
pub fn handle_path(path: &str) -> String {
    let watch_path = WATCH_PATH.get().map_or(Path::new("."), |v| v);
    
    let abs_path = if path.starts_with("/") {
        // Already absolute
        Path::new(path).to_path_buf()
    } else {
        // Relative path - make it absolute by joining with watch_path
        // Beware!! Make sure all the paths that come here are from trusted sources.
        // By making a path absolute, a path input with .. can request for 
        // the whole parent directories hash, which might be benign but might
        // still result in permission denied error or might require a lot of time
        // to collect unnecessary files.
        watch_path.join(path)
    };

    // Canonicalize the path. This may fail in case when files are renamed or deleted.
    let canonical_abs_path = abs_path.canonicalize().unwrap_or_else(|_| {
        // If canonicalize fails, manually normalize by removing . and .. components
        abs_path.components().fold(PathBuf::new(), |mut path, comp| {
            match comp {
                std::path::Component::CurDir => path, // Skip "."
                std::path::Component::ParentDir => { 
                    path.pop(); 
                    path 
                }, // Handle ".."
                _ => { 
                    path.push(comp); 
                    path 
                }
            }
        })
    });

    canonical_abs_path.to_string_lossy().to_string()
}

pub fn is_directory(path: &str) -> bool {
    Path::new(path).is_dir()
}

// Helper function to walk a directory in a file system recursively and collect all file paths
pub fn walk_directory(dir_path: &str) -> io::Result<Vec<String>> {
    let mut files = Vec::new();
    let path = std::path::Path::new(dir_path);
    
    if !path.is_dir() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, 
                                 format!("{} is not a directory", dir_path)));
    }
    
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();
        
        if path.is_dir() {
            // Recursively process subdirectories
            let subdir_path = path.to_string_lossy().to_string();
            let mut subdir_files = walk_directory(&subdir_path)?;
            files.append(&mut subdir_files);
        } else if path.is_file() {
            // Add file to list
            if let Some(path_str) = path.to_str() {
                files.push(handle_path(path_str));
            }
        }
    }
    
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use tempfile::tempdir;

    #[test]
    fn test_handle_path_variants() {
        assert_eq!(handle_path("./foo/bar.txt"), "foo/bar.txt");
        assert_eq!(handle_path("foo/bar.txt"), "foo/bar.txt");
    }

    #[test]
    fn test_is_directory_behavior() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test_file.txt");
        File::create(&file_path).unwrap();

        assert!(is_directory(dir.path().to_str().unwrap()));
        assert!(!is_directory(file_path.to_str().unwrap()));
    }

    #[test]
    fn test_walk_directory_structure() {
        let base_dir = tempdir().unwrap();
        let sub_dir = base_dir.path().join("sub");
        fs::create_dir(&sub_dir).unwrap();

        let file1_path = base_dir.path().join("file1.txt");
        File::create(&file1_path).unwrap();
        let file2_path = sub_dir.join("file2.txt");
        File::create(&file2_path).unwrap();

        let files = walk_directory(base_dir.path().to_str().unwrap()).unwrap();
        
        // handle_path will be applied by walk_directory, so check against normalized paths
        let expected_file1 = handle_path(file1_path.to_str().unwrap());
        let expected_file2 = handle_path(file2_path.to_str().unwrap());

        assert_eq!(files.len(), 2);
        assert!(files.contains(&expected_file1));
        assert!(files.contains(&expected_file2));
    }
}