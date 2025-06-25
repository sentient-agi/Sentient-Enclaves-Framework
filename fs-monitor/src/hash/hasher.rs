use sha3::{Digest, Sha3_512};
use std::io::{self, Read};
use std::fs;

// Hash a file given it's path.
// This works well with absolute paths, so unless testing
// make sure the path has been canonicalized through
// handle_path function. Relative paths might fail if
// they aren't relative to CWD of the fs_monitor.
pub fn hash_file(file_path: &str) -> io::Result<Vec<u8>> {
    let mut file = fs::File::open(file_path)?;
    let mut hasher = Sha3_512::new();
    let mut buffer = [0; 8192];

    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(hasher.finalize().to_vec())
}

pub fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter()
        .map(|b| format!("{:02x}", b))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_hash_file_simple() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "hello world").unwrap();

        let hash_result = hash_file(file_path.to_str().unwrap());
        assert!(hash_result.is_ok());

        // Calculate expected hash for "hello world\n"
        let mut expected_hasher = Sha3_512::new();
        expected_hasher.update(b"hello world\n");
        let expected_hash = expected_hasher.finalize().to_vec();

        assert_eq!(hash_result.unwrap(), expected_hash);
    }

    #[test]
    fn test_bytes_to_hex_conversion() {
        let bytes = vec![0x12, 0x34, 0xAB, 0xCD];
        assert_eq!(bytes_to_hex(&bytes), "1234abcd");
        let empty_bytes: Vec<u8> = Vec::new();
        assert_eq!(bytes_to_hex(&empty_bytes), "");
    }
}
