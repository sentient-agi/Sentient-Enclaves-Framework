
#[derive(Debug, Clone, PartialEq)]
pub enum FileType {
    File,
    // Directory,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FileState {
    Created,
    Modified,
    Closed,
    Renamed,
}


#[derive(Debug, Clone)]
pub struct FileInfo {
    pub file_type: FileType,
    pub state: FileState,
    pub version: usize,
} 
