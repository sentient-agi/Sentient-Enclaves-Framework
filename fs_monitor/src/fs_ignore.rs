use glob::Pattern;
use std::fs::File;
use std::io::{BufRead, BufReader};
pub struct IgnoreList {
    patterns: Vec<Pattern>,
}

impl IgnoreList {
    pub fn new() -> Self {
        IgnoreList {
            patterns: Vec::new(),
        }
    }

    pub fn populate_ignore_list(&mut self, file_path: &str) {
        // read a file with the ignore list and populate the ignore list
        let file = File::open(file_path).unwrap();
        let reader = BufReader::new(file);
        let lines = reader.lines();
        for line in lines {
            let line = line.unwrap();
            match Pattern::new(&line){
                Ok(pattern)=> self.patterns.push(pattern),
                Err(_) => panic!("Pattern syntax error"),
            }
            
        }
    }

    pub fn is_ignored(&self, path: &str) -> bool {
        for pattern in &self.patterns {
            if pattern.matches(path) {
                return true;
            }
        }
        false
    }
}

// This module makes it convenient to ignore some files from the watcher.
// This can be used to exclude temporarily created files like .cache while
// models are being downloaded. 

// Necessary unit tests
// 1. Test that the ignore list generated is empty
// 2. Test behavior when invalid patterns are provided
// 3. Test behavior with valid nested patter

#[cfg(test)]
mod tests{
    use std::io::Write;

    use tempfile::NamedTempFile;

    use super::*;

    fn create_temp_ignore_file(patterns: &[&str]) -> NamedTempFile{
        let mut file = NamedTempFile::new().unwrap();

        // Start adding patterns
        for pattern in patterns{
            writeln!(file,"{}",pattern).unwrap();
        }
        file.flush().unwrap();
        file
    }

    #[test]
    fn empty_ignore_list() {
        let ignore_list = IgnoreList::new();
        assert!(ignore_list.patterns.is_empty())
    }

    #[test]
    #[should_panic(expected = "Pattern syntax error")]
    fn test_invalid_pattern() {
        let patterns = vec!["[invalid-pattern"];
        let temp_file = create_temp_ignore_file(&patterns);
        
        let mut ignore_list = IgnoreList::new();
        ignore_list.populate_ignore_list(temp_file.path().to_str().unwrap());

    }

    #[test]
    fn test_valid_patterns(){
        let patterns = vec!["**/.cache/**", "**/*.swp", "**/tmp_*"];
        let temp_file = create_temp_ignore_file(&patterns);

        let mut ignore_list = IgnoreList::new();
        ignore_list.populate_ignore_list(temp_file.path().to_str().unwrap());

        assert!(ignore_list.is_ignored(".cache"));

        assert!(ignore_list.is_ignored(".cache"));
        
        assert!(ignore_list.is_ignored("foo/.cache"));
        assert!(ignore_list.is_ignored("foo/bar/.cache"));
        

        println!("Testing: .cache/file1");
        assert!(ignore_list.is_ignored(".cache/file1"));
        assert!(ignore_list.is_ignored(".cache/d1/file1"));
    }


}
