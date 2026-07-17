use std::fs::File;
use std::io::{Read, BufReader};
use std::path::Path;

use std::hash::Hasher;
use blake3::Hasher as Blake3Hasher;
use xxhash_rust::xxh3::Xxh3;

use crate::error::MetadataResult;

pub struct FileHashes {
    pub blake3: String,
    pub xxh3: String,
}

const CHUNK_SIZE: usize = 1024 * 1024; // 1MB chunks

/// Calculate both blake3 and xxh3 hashes for a file.
/// Optimised for large files by streaming in chunks.
pub fn calculate_hashes<P: AsRef<Path>>(path: P) -> MetadataResult<FileHashes> {
    let file = File::open(path)?;
    let mut reader = BufReader::with_capacity(CHUNK_SIZE * 4, file);
    
    let mut blake3_hasher = Blake3Hasher::new();
    let mut xxh3_hasher = Xxh3::new();
    
    let mut buffer = vec![0u8; CHUNK_SIZE];
    
    loop {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        blake3_hasher.update(&buffer[..bytes_read]);
        xxh3_hasher.write(&buffer[..bytes_read]);
    }
    
    let blake3_hash = blake3_hasher.finalize().to_hex().to_string();
    let xxh3_hash = format!("{:016x}", xxh3_hasher.finish());
    
    Ok(FileHashes {
        blake3: blake3_hash,
        xxh3: xxh3_hash,
    })
}
