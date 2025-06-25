#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum FileType {
    File,
    Directory,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FileState {
    Created,
    Modified,
    Closed,
    // Deleted
}


#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct FileInfo {
    pub file_type: FileType,
    pub state: FileState,
    pub version: usize,
}
