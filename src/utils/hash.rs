use std::path::Path;
use blake3::Hasher;
use sha2::{Sha256, Digest};
use tokio::io::{AsyncReadExt, BufReader};

use crate::core::error::{DotmanError, Result};

/// Hash type enumeration
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HashType {
    Blake3,
    Sha256,
}

/// Hash utility for file integrity checking
pub struct HashUtility;

impl HashUtility {
    /// Calculate BLAKE3 hash of a file
    pub async fn blake3_file(path: &Path) -> Result<String> {
        let file = tokio::fs::File::open(path).await
            .map_err(|e| DotmanError::io(format!("Failed to open file for hashing: {}", e)))?;
        
        let mut reader = BufReader::new(file);
        let mut hasher = Hasher::new();
        let mut buffer = vec![0u8; 8192];
        
        loop {
            let bytes_read = reader.read(&mut buffer).await
                .map_err(|e| DotmanError::io(format!("Failed to read file: {}", e)))?;
            
            if bytes_read == 0 {
                break;
            }
            
            hasher.update(&buffer[..bytes_read]);
        }
        
        Ok(hasher.finalize().to_hex().to_string())
    }

    /// Calculate SHA256 hash of a file
    pub async fn sha256_file(path: &Path) -> Result<String> {
        let file = tokio::fs::File::open(path).await
            .map_err(|e| DotmanError::io(format!("Failed to open file for hashing: {}", e)))?;
        
        let mut reader = BufReader::new(file);
        let mut hasher = Sha256::new();
        let mut buffer = vec![0u8; 8192];
        
        loop {
            let bytes_read = reader.read(&mut buffer).await
                .map_err(|e| DotmanError::io(format!("Failed to read file: {}", e)))?;
            
            if bytes_read == 0 {
                break;
            }
            
            hasher.update(&buffer[..bytes_read]);
        }
        
        Ok(format!("{:x}", hasher.finalize()))
    }

    /// Calculate hash based on hash type
    pub async fn hash_file(path: &Path, hash_type: HashType) -> Result<String> {
        match hash_type {
            HashType::Blake3 => Self::blake3_file(path).await,
            HashType::Sha256 => Self::sha256_file(path).await,
        }
    }

    /// Verify file integrity against expected hash
    pub async fn verify_integrity(path: &Path, expected_hash: &str, hash_type: HashType) -> Result<bool> {
        let actual_hash = Self::hash_file(path, hash_type).await?;
        Ok(actual_hash == expected_hash)
    }

    /// Calculate hash of bytes
    pub fn blake3_bytes(data: &[u8]) -> String {
        blake3::hash(data).to_hex().to_string()
    }

    /// Calculate SHA256 hash of bytes
    pub fn sha256_bytes(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        format!("{:x}", hasher.finalize())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;

    #[tokio::test]
    async fn test_file_hashing() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let test_data = b"Hello, world!";
        temp_file.write_all(test_data).unwrap();
        temp_file.flush().unwrap();

        let blake3_hash = HashUtility::blake3_file(temp_file.path()).await.unwrap();
        let sha256_hash = HashUtility::sha256_file(temp_file.path()).await.unwrap();

        // Verify hashes are not empty
        assert!(!blake3_hash.is_empty());
        assert!(!sha256_hash.is_empty());

        // Verify integrity
        assert!(HashUtility::verify_integrity(temp_file.path(), &blake3_hash, HashType::Blake3).await.unwrap());
        assert!(HashUtility::verify_integrity(temp_file.path(), &sha256_hash, HashType::Sha256).await.unwrap());

        // Test with wrong hash
        assert!(!HashUtility::verify_integrity(temp_file.path(), "wrong_hash", HashType::Blake3).await.unwrap());
    }

    #[test]
    fn test_bytes_hashing() {
        let test_data = b"Hello, world!";
        
        let blake3_hash = HashUtility::blake3_bytes(test_data);
        let sha256_hash = HashUtility::sha256_bytes(test_data);

        assert!(!blake3_hash.is_empty());
        assert!(!sha256_hash.is_empty());
        
        // Verify consistency
        assert_eq!(blake3_hash, HashUtility::blake3_bytes(test_data));
        assert_eq!(sha256_hash, HashUtility::sha256_bytes(test_data));
    }
} 