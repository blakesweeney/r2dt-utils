use std::path::{Path, PathBuf};
use std::str;

use anyhow::{anyhow, Result};

use regex::Regex;

pub fn directory_path(base: &PathBuf, urs: &String) -> PathBuf {
    let mut path = PathBuf::from(base);
    path.push("URS");
    for x in (3..11).step_by(2) {
        path.push(urs[x..(x + 2)].to_string());
    }
    return path;
}

pub fn uncompressed_path(base: &PathBuf, urs: &String) -> PathBuf {
    let mut path = directory_path(&base, &urs);
    path.push(urs);
    path.set_extension("svg");
    return path;
}

pub fn path_for(base: &PathBuf, urs: &String) -> PathBuf {
    let mut path = PathBuf::from(uncompressed_path(base, urs));
    path.set_extension("svg.gz");
    return path;
}

pub fn incorrect_paths(base: &PathBuf, urs: &String) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let mut standard_path = path_for(base, urs);
    standard_path.set_extension(".svg.gz");
    paths.push(standard_path);
    return paths;
}

pub fn looks_like_urs(urs: &str) -> bool {
    lazy_static! {
        static ref PATTERN: Regex = Regex::new(r"URS[0-9A-F]{10}$").unwrap();
    }
    return PATTERN.is_match(urs);
}

pub fn filename_urs(urs: &Path) -> Option<String> {
    lazy_static! {
        static ref MODEL_SUFFIX: Regex = Regex::new(r"-.+$").unwrap();
    }

    return urs
        .file_name()
        .and_then(|f| f.to_str())
        .map(|s| s.replace(".gz", ""))
        .map(|s| s.replace("..svg", ""))
        .map(|s| s.replace(".svg", ""))
        .map(|s| s.replace(".colored", ""))
        .map(|s| MODEL_SUFFIX.replace(&s, "").to_string())
        .and_then(|s| match looks_like_urs(&s) {
            true => Some(s.to_string()),
            false => None,
        });
}

pub fn int_to_urs(index: usize) -> String {
    return format!("URS{:010X}", index);
}

pub fn urs_to_index(urs: &String) -> Result<usize> {
    let minimal = urs.replace("URS", "");
    let result = usize::from_str_radix(&minimal, 16);
    match result {
        Ok(r) => Ok(r),
        Err(_) => Err(anyhow!("Could not parse {}", urs)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_urs() {
        assert_eq!(looks_like_urs("URS00000001AAB82D"), false);
        assert_eq!(looks_like_urs("URS00000001B1"), true);
        assert_eq!(looks_like_urs("URS0000000362"), true);
    }

    #[test]
    fn extracts_urs() {
        assert_eq!(
            filename_urs(Path::new("a/b/URS0000000372..svg.gz")),
            Some("URS0000000372".to_string())
        );
        assert_eq!(
            filename_urs(Path::new("URS0000000372.svg.gz")),
            Some("URS0000000372".to_string())
        );
        assert_eq!(
            filename_urs(Path::new("URS0000000372.svg")),
            Some("URS0000000372".to_string())
        );
        assert_eq!(
            filename_urs(Path::new("URS0000000372")),
            Some("URS0000000372".to_string())
        );
        assert_eq!(
            filename_urs(Path::new("URS000042DD9D.colored.svg")),
            Some("URS000042DD9D".to_string())
        );
        assert_eq!(filename_urs(Path::new("URS00000002D191B..svg.gz")), None);
        assert_eq!(filename_urs(Path::new("URS00000002C67ED..svg.gz")), None);
        assert_eq!(filename_urs(Path::new("URS00000002C67ED..svg")), None);
        assert_eq!(filename_urs(Path::new("URS00000002C67ED.")), None);
        assert_eq!(filename_urs(Path::new("URS00000002C67ED")), None);
        assert_eq!(
            filename_urs(Path::new("URS0000C2D164-E-Ser.colored.svg")),
            Some("URS0000C2D164".to_string())
        );
    }

    #[test]
    fn creates_correct_final_path() {
        let mut result = PathBuf::from("foo");
        result.push("URS");
        result.push("00");
        result.push("00");
        result.push("00");
        result.push("03");
        result.push("URS0000000372");
        result.set_extension("svg.gz");
        assert_eq!(
            path_for(&PathBuf::from("foo"), &"URS0000000372".to_string()),
            result
        );
    }

    #[test]
    fn correctly_generates_urs() {
        assert_eq!(int_to_urs(12601134), String::from("URS0000C0472E"));
        assert_eq!(int_to_urs(1), String::from("URS0000000001"));
        assert_eq!(int_to_urs(9), String::from("URS0000000009"));
    }

    #[test]
    fn correctly_generates_urs_index() -> Result<()> {
        assert_eq!(urs_to_index(&String::from("URS0000000009"))?, 9);
        assert_eq!(urs_to_index(&String::from("URS0000C0472E"))?, 12601134);
        assert_eq!(urs_to_index(&String::from("URS0000000001"))?, 1);
        return Ok(());
    }
}
