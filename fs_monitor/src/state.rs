#[derive(Debug, Clone, PartialEq)]
pub enum FileType {
    File,
    // Directory,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FileState {
    Created,
    Modified,
    Immutable,
    // Deleted,
}

// #[derive(Debug, Clone)]
// pub struct HashInfo {
//     pub hash_state: HashState,
//     pub hash: Option<String>,
// }

// #[derive(Debug, Clone)]
// pub enum HashState {
//     InProgress,
//     Complete,
//     Error,
// }

#[derive(Debug, Clone)]
pub struct FileInfo {
    pub file_type: FileType,
    pub state: FileState, 
    pub hash: Option<String>,
    // pub hash_info: Option<HashInfo>, // Relevant for files
} 