use std::path::Path;
use std::io;

// This normalizes paths returned by notify crate
// for internal use. Make sure this function is called
// always before using path with internal data structures.
pub fn handle_path(path: &str) -> String {
    // check if path is absolute
    // if it is then make it relative
    // with respect to the current working directory
    if path.starts_with("/") {
        let current_dir = std::env::current_dir().unwrap();
        let relative_path = Path::new(&path).strip_prefix(current_dir).unwrap();
        // if there is ./ in the path then remove it
        let relative_path = if relative_path.starts_with("./") {
            relative_path.strip_prefix("./").unwrap()
        } else {
            relative_path
        };
        // update the path with the relative path
        let path = relative_path.to_str().unwrap();
        // eprintln!("Relative path: {}", path);
        path.to_string()
    } else if path.starts_with("./"){
            let path = path.strip_prefix("./").unwrap();
            eprintln!("Striped Path: {:?}", path);
            path.to_string()
    } else {
        path.to_string()
    }
}

pub fn is_directory(path: &str) -> bool {
    Path::new(path).is_dir()
}

// Helper function to walk a directory recursively and collect all file paths
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