use std::path::{Path, PathBuf};

pub fn path_for(base: &Path, urs: String) -> PathBuf {
    return PathBuf::from(base);
}

pub fn is_uncompressed_path(base: &Path, urs: String, found: &Path) -> bool {
    return false;
}

pub fn incorrect_paths(base: &Path, urs: String) -> Vec<PathBuf> {
    return Vec::new();
}

pub fn looks_like_urs(urs: String) -> bool {
    return false;
}
