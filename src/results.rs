use std::fs::read_to_string;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::path::PathBuf;
use std::collections::HashMap;

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

struct DiagramSvg {
    urs: String,
    path: PathBuf,
}

#[derive(Debug,Deserialize)]
struct UrsRename {
    old_urs: String,
    new_urs: String,
}

enum Renamer {
    NoRename,
    UseMapping(HashMap<String, String>),
}

impl Renamer {
    pub fn new(filename: Option<PathBuf>) -> Result<Self> {
        return match filename {
            None => Ok(Self::NoRename),
            Some(filename) => {
                let file = File::open(filename)?;
                let reader = BufReader::new(file);
                let mut reader = csv::Reader::from_reader(reader);

                let mut mapping = HashMap::new();
                for record in reader.deserialize() {
                    let record: UrsRename = record?;
                    mapping.insert(record.old_urs, record.new_urs);
                }
                return Ok(Self::UseMapping(mapping));
            },
        }
    }

    pub fn rename(&self, urs: &String) -> Option<String> {
        return match self {
            Self::NoRename => Some(urs.to_string()),
            Self::UseMapping(mapping) => mapping.get(urs).map(|s| s.to_string()),
        }
    }
}

fn write(diagram: &JsonDiagram, renamer: &Renamer, base: &PathBuf) -> Result<()> {
    let urs = renamer.rename(&diagram.urs);
    if urs.is_none() {
        log::error!("Could not find renamed URS for {:?}", urs);
        return Ok(());
    };
    let urs = urs.unwrap();
    let path = urs_utils::path_for(base, &urs);
    let out_file = File::create(path)?;
    let mut gz = GzEncoder::new(out_file, Compression::default());
    gz.write_all(&diagram.svg.as_ref())?;
    return Ok(());
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

pub fn move_file(filename: PathBuf, target_directory: PathBuf, mapping_file: Option<PathBuf>) -> Result<()> {
    let file = File::open(filename)?;
    let reader = BufReader::new(file);
    let renamer = Renamer::new(mapping_file)?;

    for line in reader.lines() {
        let line = line?;
        let line_path = PathBuf::from(line);
        for diagram in svgs(line_path)? {
            let svg_text = read_to_string(&diagram.path)?;
            let json = JsonDiagram { urs: diagram.urs, svg: svg_text };
            write(&json, &renamer, &target_directory)?;
        }
    }

    return Ok(());
}

pub fn split_file(filename: PathBuf, target_directory: PathBuf, mapping_file: Option<PathBuf>) -> Result<()> {
    let file = File::open(filename)?;
    let file = BufReader::new(file);
    let renamer = Renamer::new(mapping_file)?;
    for line in file.lines() {
        let line = line?.replace("\\\\", "\\");
        let entry: JsonDiagram = serde_json::from_str(&line)?;
        write(&entry, &renamer, &target_directory)?;
    }
    return Ok(());
}
