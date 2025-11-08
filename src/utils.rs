use blake3::Hash;

pub(crate) fn hash_file(path: impl AsRef<std::path::Path>) -> Hash {
    let mut hasher = blake3::Hasher::new();
    match hasher.update_mmap(path) {
        Ok(_) => {}
        Err(e) => panic!("Failed to hash file: {}", e),
    }
    let hash = hasher.finalize();
    hash
}
