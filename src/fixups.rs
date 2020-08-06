use std::collections::HashSet;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use walkdir::WalkDir;

use anyhow::Result;

pub mod urs_utils;

#[derive(Serialize, Deserialize, Debug)]
enum UrsStatus {
    CorrectSvg {
        urs: String,
    },
    MissingSvg {
        urs: String,
    },
    ExtraSvg {
        urs: String,
        found_at: PathBuf,
    },
    RenameSvg {
        urs: String,
        found_at: PathBuf,
        expected_path: PathBuf,
    },
    CompressSvg {
        urs: String,
        found_at: PathBuf,
        expected_path: PathBuf,
    },
    UnknownFile {
        path: PathBuf,
    },
}

fn load_required(path: PathBuf) -> Result<HashSet<String>> {
    let mut known = HashSet::new();
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    for line in reader.lines() {
        let line = line?.trim().to_owned();
        known.insert(line);
    }
    return Ok(known);
}

fn compare_paths(base: &PathBuf, urs: &String, path: &Path) -> UrsStatus {
    let expected_path = urs_utils::path_for(&base, urs);
    if expected_path == path {
        return UrsStatus::CorrectSvg {
            urs: urs.to_string(),
        };
    }

    let uncompressed_path = urs_utils::uncompressed_path(&base, &urs);
    if uncompressed_path == path {
        return UrsStatus::CompressSvg {
            urs: urs.to_string(),
            found_at: PathBuf::from(path),
            expected_path,
        };
    }

    for possible in urs_utils::incorrect_paths(&base, &urs) {
        if possible == path {
            return UrsStatus::RenameSvg {
                urs: urs.to_string(),
                found_at: possible,
                expected_path,
            };
        }
    }

    return UrsStatus::UnknownFile {
        path: PathBuf::from(path),
    };
}

pub fn write_report(base: &PathBuf, required_file: PathBuf) -> Result<()> {
    let mut required = load_required(required_file)?;
    let walker = WalkDir::new(PathBuf::from(base))
        .into_iter()
        .filter_map(Result::ok);

    for dir_entry in walker {
        let path = dir_entry.path();
        let status = match urs_utils::filename_urs(&path) {
            None => UrsStatus::UnknownFile {
                path: PathBuf::from(path),
            },
            Some(urs) => match required.remove(&urs) {
                true => compare_paths(&base, &urs, &path),
                false => UrsStatus::UnknownFile {
                    path: PathBuf::from(path),
                },
            },
        };

        let json = serde_json::to_string(&status)?;
        println!("{}", json);
    }

    for urs in required {
        let status = UrsStatus::MissingSvg { urs };
        let json = serde_json::to_string(&status)?;
        println!("{}", json);
    }

    return Ok(());
}
