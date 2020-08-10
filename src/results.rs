use std::fs::read_to_string;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::path::PathBuf;

use flate2::write::GzEncoder;
use flate2::Compression;

use serde::{Deserialize, Serialize};

use anyhow::Result;

use globset::Glob;
use walkdir::WalkDir;

use crate::fixups::urs_utils;

#[derive(Serialize, Deserialize, Debug)]
pub struct JsonDiagram {
    pub urs: String,
    pub svg: String,
}

impl JsonDiagram {
    pub fn dump(self, base: &PathBuf) -> Result<()> {
        let path = urs_utils::path_for(&base, &self.urs);
        let mut out = File::create(path)?;
        out.write_all(&self.svg.as_bytes())?;
        return Ok(());
    }
}

struct DiagramSvg {
    urs: String,
    path: PathBuf,
}

impl DiagramSvg {
    pub fn final_path(self, base: &PathBuf) -> PathBuf {
        return urs_utils::path_for(&base, &self.urs);
    }
}

fn svgs(directory: PathBuf) -> Result<Vec<DiagramSvg>> {
    let mut result_path = PathBuf::from(directory);
    result_path.push("output");
    result_path.push("results");
    let glob = Glob::new("URS*.svg")?.compile_matcher();
    let svgs = WalkDir::new(result_path)
        .into_iter()
        .filter_map(Result::ok)
        .map(|f| PathBuf::from(f.path()))
        .filter(|f| glob.is_match(f))
        .filter_map(|path| urs_utils::filename_urs(&path).map(|urs| DiagramSvg { urs, path }))
        .collect::<Vec<DiagramSvg>>();
    return Ok(svgs);
}

pub fn move_file(filename: PathBuf, target_directory: PathBuf) -> Result<()> {
    let file = File::open(filename)?;
    let reader = BufReader::new(file);

    for line in reader.lines() {
        let line = line?;
        let line_path = PathBuf::from(line);
        for svg in svgs(line_path)? {
            let svg_text = read_to_string(&svg.path)?;
            let final_path = svg.final_path(&target_directory);
            let out_file = File::create(final_path)?;
            let mut gz = GzEncoder::new(out_file, Compression::default());
            gz.write_all(&svg_text.as_ref())?;
        }
    }

    return Ok(());
}

pub fn split_file(filename: PathBuf, target_directory: PathBuf) -> Result<()> {
    let file = File::open(filename)?;
    let file = BufReader::new(file);
    for line in file.lines() {
        let line = line?.replace("\\\\", "\\");
        let entry: JsonDiagram = serde_json::from_str(&line)?;
        entry.dump(&target_directory)?;
    }
    return Ok(());
}
