use std::collections::HashMap;
use std::fs::read_to_string;
use std::fs::File;
use std::io::prelude::*;
use std::io::{stdout, BufReader};
use std::path::PathBuf;
use std::time::SystemTime;

use flate2::write::GzEncoder;
use flate2::Compression;

use serde::{Deserialize, Serialize};

use anyhow::Result;

use walkdir::WalkDir;

use crate::fixups::urs_utils;

#[derive(Serialize, Deserialize, Debug)]
pub struct JsonDiagram {
    pub urs: String,
    pub svg: String,
}

#[derive(Debug)]
struct DiagramSvg {
    urs: String,
    path: PathBuf,
}

#[derive(Debug, Deserialize)]
struct UrsRename {
    old_urs: String,
    new_urs: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct Metadata {
    urs: String,
    secondary_structure: String,
    overlap_count: u64,
    basepair_count: u64,
    model_start: Option<u64>,
    model_stop: Option<u64>,
    sequence_start: Option<u64>,
    sequence_stop: Option<u64>,
    sequence_coverage: Option<f64>,
    model_name: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct Cdmi {
    value: String,
}

enum Renamer {
    NoRename,
    UseMapping(HashMap<String, String>),
}

pub struct TransferOptions {
    pub host: String,
    pub access_token: String,
    pub remote_path: String,
    pub use_http: bool,
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
            }
        };
    }

    pub fn rename(&self, urs: &String) -> Option<String> {
        return match self {
            Self::NoRename => Some(urs.to_string()),
            Self::UseMapping(mapping) => mapping.get(urs).map(|s| s.to_string()),
        };
    }
}

impl Cdmi {
    pub fn from_path(path: &PathBuf) -> Result<Self> {
        let mut buf = Vec::new();
        let mut file = File::open(path)?;
        file.read_to_end(&mut buf)?;
        let value = base64::encode(buf);
        return Ok(Self { value });
    }
}

impl TransferOptions {
    pub fn scheme(&self) -> &str {
        return match self.use_http {
            true => "http",
            false => "https",
        };
    }
}

fn write(diagram: &JsonDiagram, renamer: &Renamer, base: &PathBuf) -> Result<()> {
    let urs = renamer.rename(&diagram.urs);
    if urs.is_none() {
        log::error!("Could not find renamed URS for {:?}", urs);
        return Ok(());
    };
    let urs = urs.unwrap();
    log::info!("Renaming {} to {}", &diagram.urs, &urs);
    let path = urs_utils::path_for(base, &urs);
    log::info!("Writing to {:?}", &path);
    let out_file = File::create(path)?;
    let mut gz = GzEncoder::new(out_file, Compression::default());
    gz.write_all(&diagram.svg.as_ref())?;
    return Ok(());
}

fn svgs(directory: PathBuf) -> Result<Vec<DiagramSvg>> {
    let mut svgs = Vec::new();
    for filename in WalkDir::new(directory) {
        log::debug!("Found file: {:?}", &filename);
        let filename = filename?;
        let path = PathBuf::from(filename.path());
        let base = path.file_name();
        if base.is_none() {
            log::debug!("Skipping as no filename");
        }
        let base = base.unwrap();
        let urs = urs_utils::filename_urs(&PathBuf::from(base));
        if urs.is_none() {
            log::debug!("Cannot extract urs from {:?}", &path);
            continue;
        }
        let urs = urs.unwrap();
        svgs.push(DiagramSvg { urs, path });
    }
    return Ok(svgs);
}

pub fn move_file(
    filename: PathBuf,
    target_directory: PathBuf,
    mapping_file: Option<PathBuf>,
) -> Result<()> {
    let file = File::open(filename)?;
    let reader = BufReader::new(file);
    let renamer = Renamer::new(mapping_file)?;

    for line in reader.lines() {
        let line = line?;
        let line_path = PathBuf::from(line);
        for diagram in svgs(line_path)? {
            log::info!("Moving {} found at {:?}", &diagram.urs, &diagram.path);
            let svg_text = read_to_string(&diagram.path)?;
            let json = JsonDiagram {
                urs: diagram.urs,
                svg: svg_text,
            };
            write(&json, &renamer, &target_directory)?;
        }
    }

    return Ok(());
}

pub fn split_file(
    filename: PathBuf,
    target_directory: PathBuf,
    mapping_file: Option<PathBuf>,
) -> Result<()> {
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

pub fn rename_metadata(mapping_file: PathBuf, filename: PathBuf) -> Result<()> {
    let renamer = Renamer::new(Some(mapping_file))?;
    let file = File::open(filename)?;
    let reader = BufReader::new(file);
    let mut reader = csv::Reader::from_reader(reader);
    let mut writer = csv::Writer::from_writer(stdout());
    let mut unmapped = 0;
    for record in reader.deserialize() {
        let record: Metadata = record?;
        let urs = renamer.rename(&record.urs);
        if urs.is_none() {
            log::error!("Could not find renamed URS for {:?}", &record.urs);
            unmapped += 1;
            continue;
        };
        let record = Metadata {
            urs: urs.unwrap(),
            ..record
        };
        writer.serialize(record)?;
    }
    writer.flush()?;
    log::info!("Did not find mapping for {} urs ids", unmapped);
    return Ok(());
}

pub fn transfer_svgs(filename: &PathBuf, options: TransferOptions) -> Result<()> {
    let file = File::open(filename)?;
    let file = BufReader::new(file);
    let client = reqwest::blocking::Client::new();
    let start = SystemTime::now();
    let mut total: usize = 0;
    for line in file.lines() {
        total += 1;
        let line = line?;
        let path = PathBuf::from(&line);
        if !path.is_file() {
            log::error!("Could not load SVG file {:?}", &path);
            continue;
        }
        let urs = path.file_name().and_then(|s| s.to_str());
        if urs.is_none() {
            log::error!("No URS found in filename: {:?}", &path);
            continue;
        }
        let urs = urs.unwrap();
        let url = format!(
            "{}://{}/cdmi/RNA-Sequences/{}/{}",
            options.scheme(), &options.host, options.remote_path, &urs,
        );
        let object = Cdmi::from_path(&path)?;
        log::debug!("Sending {:?} to {}", &path, &url);
        client
            .put(&url)
            .header("X-Auth-Token", &options.access_token)
            .header("X-CDMI-Specification-Version", "1.1.1")
            .header("Content-Type", "application/cdmi-object")
            .body(serde_json::to_string(&object)?)
            .send()?;
        if total % 10 == 0 {
            log::info!(
                "Sent {} requests in {} milliseconds",
                total,
                start.elapsed()?.as_millis()
            );
        }
    }

    log::info!(
        "Sent {} requests in {} milliseconds",
        total,
        start.elapsed()?.as_millis()
    );

    return Ok(());
}
