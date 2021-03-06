use std::fs::File;
use std::io::prelude::*;
use std::io::{self, BufReader};
use std::iter::Iterator;
use std::path::PathBuf;

use globset::{Glob, GlobSetBuilder};
use walkdir::WalkDir;

use log::{info, trace};

use quick_xml::events::Event;
use quick_xml::Reader;

use flate2::read::GzDecoder;

use serde::{Deserialize, Serialize};

use anyhow::{anyhow, Result};

use crate::fixups::urs_utils;

use crate::results::JsonDiagram;

#[derive(Serialize, Deserialize, Debug)]
struct Counts {
    urs: String,
    changed: u64,
    unchanged: u64,
    inserted: u64,
    moved: u64,
    rotated: u64,
    total: u64,
}

fn is_valid_letter(text: String) -> bool {
    match text.as_ref() {
        "A" | "C" | "G" | "U" | "X" => true,
        _ => false,
    }
}

fn count_reader<B: BufRead>(urs: String, reader: &mut Reader<B>) -> Result<Counts> {
    let mut counts = Counts {
        urs,
        changed: 0,
        unchanged: 0,
        inserted: 0,
        moved: 0,
        rotated: 0,
        total: 0,
    };
    let mut buf = Vec::new();
    loop {
        match reader.read_event(&mut buf) {
            Ok(Event::Start(ref e)) => match e.name() {
                b"text" => {
                    let text = reader.read_text(e.name(), &mut Vec::new())?;
                    let class = e.attributes().with_checks(false).find(|attr| match attr {
                        Ok(a) => a.key == b"class",
                        _ => false,
                    });
                    match (is_valid_letter(text), class) {
                        (true, Some(Ok(v))) => match v.value.as_ref() {
                            b"green" => counts.changed += 1,
                            b"black" => counts.unchanged += 1,
                            b"red" => counts.inserted += 1,
                            b"blue" => counts.moved += 1,
                            b"brown" => counts.rotated += 1,
                            _ => (),
                        },
                        _ => (),
                    }
                }
                _ => (),
            },
            Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
            Ok(Event::Eof) => break,
            _ => (),
        }
    }
    counts.total =
        counts.changed + counts.unchanged + counts.inserted + counts.moved + counts.rotated;
    return Ok(counts);
}

fn count_path(path: &PathBuf) -> Result<Counts> {
    let urs = match urs_utils::filename_urs(&path) {
        Some(u) => Ok(u),
        None => Err(anyhow!("SVG does not have a URS")),
    }?;

    info!("Parsing data for {:?}", path);
    return match path.extension() {
        Some(ext) => match ext.to_str() {
            Some("svg") => {
                trace!("Parsing as svg");
                let mut reader = Reader::from_file(path)?;
                return count_reader(urs, &mut reader);
            }
            Some("gz") => {
                trace!("Parsing as compressed svg");
                let file = File::open(path)?;
                let decoder = GzDecoder::new(file);
                let buf = BufReader::new(decoder);
                let mut reader = Reader::from_reader(buf);
                return count_reader(urs, &mut reader);
            }
            e => Err(anyhow!("Cannot parse file with {:?} extension", e)),
        },
        None => Err(anyhow!("File {:?} does not have an extension", path)),
    };
}

pub fn count_tree(path: PathBuf) -> Result<()> {
    let walker = WalkDir::new(path);
    let mut builder = GlobSetBuilder::new();
    builder.add(Glob::new("*.svg")?);
    builder.add(Glob::new("*.svg.gz")?);
    let glob = builder.build()?;

    let counts = walker
        .into_iter()
        .filter_map(Result::ok)
        .filter(move |e| glob.is_match(e.file_name()))
        .map(|path| count_path(&path.into_path()));

    let mut wtr = csv::Writer::from_writer(io::stdout());
    for count in counts {
        let c = count?;
        wtr.serialize(c)?;
    }
    return Ok(());
}

pub fn count_json(filename: PathBuf) -> Result<()> {
    let file = File::open(filename)?;
    let file = BufReader::new(file);
    let counts = file.lines().into_iter().map(|line| {
        let line = line?.replace("\\\\", "\\");
        let entry: JsonDiagram = serde_json::from_str(&line)?;
        let mut reader = Reader::from_str(entry.svg.as_ref());
        return count_reader(entry.urs, &mut reader);
    });
    let mut wtr = csv::Writer::from_writer(io::stdout());
    for count in counts {
        let c = count?;
        wtr.serialize(c)?;
    }
    return Ok(());
}
