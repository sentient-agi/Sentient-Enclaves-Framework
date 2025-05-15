use std::path::Path;
use std::io;

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

// Helper function to collect all files in a directory recursively
pub fn collect_files_recursively(dir_path: &std::path::Path, files: &mut Vec<String>) -> io::Result<()> {
    if !dir_path.is_dir() {
        return Ok(());
    }

    for entry in std::fs::read_dir(dir_path)? {
        let entry = entry?;
        let path = entry.path();
        
        if path.is_file() {
            if let Some(path_str) = path.to_str() {
                // Convert to the consistent path format used by the rest of the app
                let normalized_path = handle_path(path_str);
                files.push(normalized_path);
            }
        } else if path.is_dir() {
            collect_files_recursively(&path, files)?;
        }
    }
    
    Ok(())
}