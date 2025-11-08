use std::iter::repeat;
use blake3::Hash;
use std::path::Path;

pub(crate) fn hash_file<P>(path: P) -> Hash
where
    P: AsRef<Path>,
{
    let mut hasher = blake3::Hasher::new();
    match hasher.update_mmap(path) {
        Ok(_) => {}
        Err(e) => panic!("Failed to hash file: {}", e),
    }
    let hash = hasher.finalize();
    hash
}

pub(crate) fn placeholders(count: usize) -> String {
    repeat("?").take(count).collect::<Vec<_>>().join(", ")
}
