use std::path::{Path, PathBuf};

use regex::Regex;

pub fn uncompressed_path(base: &PathBuf, urs: &String) -> PathBuf {
    let mut path = PathBuf::from(base);
    path.push("URS");
    for x in (4..11).step_by(2) {
        path.push(urs[x..(x + 2)].to_string());
    }
    path.push(urs);
    path.set_extension("svg");
    return path;
}

pub fn path_for(base: &PathBuf, urs: &String) -> PathBuf {
    let mut path = PathBuf::from(uncompressed_path(base, urs));
    path.set_extension("gz");
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
    let pattern = Regex::new(r"URS[0-9A-F]{10}$");
    return match pattern {
        Ok(re) => re.is_match(urs),
        Err(_) => panic!("Someone should fix his regexs"),
    };
}

pub fn filename_urs(urs: &Path) -> Option<String> {
    return urs.file_stem().and_then(|fs| fs.to_str()).and_then(|s| {
        let prefix = &s[0..13];
        return match looks_like_urs(prefix) {
            true => Some(prefix.to_string()),
            false => None,
        };
    });
}
