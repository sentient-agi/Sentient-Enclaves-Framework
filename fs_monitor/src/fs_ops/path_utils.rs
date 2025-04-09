use std::path::Path;

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
    }
    else {
        path.to_string()
    }    
} 