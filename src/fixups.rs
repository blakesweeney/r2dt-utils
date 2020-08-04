use std::error::Error;
use std::path::{Path, PathBuf};
use std::io::{BufReader};
use std::io::prelude::*;
use std::fs::File;

use serde::{Deserialize, Serialize};

use walkdir::{DirEntry, WalkDir};

use indexmap::set::IndexSet;

mod urs_utils;

#[derive(Serialize, Deserialize, Debug)]
enum UrsStatus {
    CorrectSvg { urs: String },
    MissingSvg { urs: String },
    ExtraSvg { urs: String, path: PathBuf },
    RenameSvg { urs: String, found_at: PathBuf },
    CompressSvg { urs: String, path: PathBuf },
    UnknownFile { path: PathBuf },
}

struct StatusIterator {
    walker: walkdir::IntoIter,
    required: IndexSet<String>,
    base: PathBuf,
}

impl StatusIterator {
    fn new(base: &Path, required: IndexSet<String>) -> StatusIterator {
        let walker = WalkDir::new(PathBuf::from(base)).into_iter();

        return StatusIterator { walker, required, base: PathBuf::from(base), };
    }

    fn compare_paths(&self, urs: String, path: PathBuf) -> UrsStatus {
        let urs_path = urs_utils::path_for(&self.base, &urs);
        if urs_path == path {
            return UrsStatus::CorrectSvg { urs };
        }

        if urs_utils::is_uncompressed_path(&self.base, urs, &path) {
            return UrsStatus::CompressSvg { urs, path };
        }

        for possible in urs_utils::incorrect_paths(&self.base, urs) {
            if possible == path {
                return UrsStatus::RenameSvg { urs, found_at: possible }
            }
        }
        return UrsStatus::UnknownFile { path };
    }

    fn next_required_status(&mut self) -> Option<UrsStatus> {
        return match self.required.pop() {
            None => None,
            Some(urs) => match urs_utils::path_for(&self.base, urs).exists() {
                true => Some(UrsStatus::CorrectSvg { urs }),
                false => Some(UrsStatus::MissingSvg { urs }),
            }
        }
    }

    fn next_tree_status(&mut self, dir_entry: DirEntry) -> Option<UrsStatus> {
        let path = dir_entry.into_path();
        let urs_prefix = dir_entry.file_name().to_str()?[0..13].to_owned();
        return match urs_utils::looks_like_urs(urs_prefix) {
            false => Some(UrsStatus::UnknownFile { path }),
            true => match self.required.contains(&urs_prefix) {
                true => {
                    self.required.remove(&urs_prefix);
                    return Some(self.compare_paths(urs_prefix, path));
                },
                false => Some(UrsStatus::UnknownFile { path }),
            },
        }
    }
}

impl Iterator for StatusIterator {
    type Item = UrsStatus;

    fn next(&mut self) -> Option<UrsStatus> {
        return match self.walker.filter_map(Result::ok).next() {
            Some(v) => self.next_tree_status(v),
            None => self.next_required_status()
        }
    }
}

fn load_required(path: PathBuf) -> Result<IndexSet<String>, Box<dyn Error>> {
    let mut known = IndexSet::new();
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    for line in reader.lines() {
        let line = line?.trim().to_owned();
        known.insert(line);
    }
    return Ok(known);
}

pub fn report(base: PathBuf, required_file: PathBuf) -> Result<(), Box<dyn Error>> {
    let required = load_required(required_file)?;
    let statuses = StatusIterator::new(base.as_path(), required);
    for status in statuses {
        let json = serde_json::to_string(&status)?;
        println!("{}", json);
    }
    return Ok(());
}
