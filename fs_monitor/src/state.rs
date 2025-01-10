#[derive(Debug, Clone, PartialEq)]
pub enum FileType {
    File,
    // Directory,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FileState {
    Created,
    Modified,
    // Immutable,
    Closed,
    Renamed,
    // Deleted,
}

#[derive(Debug, Clone)]
pub struct HashInfo {
    pub hash_state: HashState,
    pub hash_string: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum HashState {
    InProgress,
    Complete,
    Error,
}

#[derive(Debug, Clone)]
pub struct FileInfo {
    pub file_type: FileType,
    pub state: FileState,
    pub hash_info: Option<HashInfo>,
    pub version: i32,
} 
