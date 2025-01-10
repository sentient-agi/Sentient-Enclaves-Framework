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
            let pattern = Pattern::new(&line.unwrap()).unwrap();
            self.patterns.push(pattern);
        }
    }

    pub fn is_ignored(&self, path: &str) -> bool {
        // eprintln!("Checking path: '{}' against patterns", path);
        for pattern in &self.patterns {
            if pattern.matches(path) {
                // eprintln!("{} -> Matched to {}", path, pattern);
                return true;
            }
        }
        false
    }
}