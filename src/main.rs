use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use std::io::{self, BufReader};
use std::iter::Iterator;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;
use globset::Glob;

use quick_xml::Reader;
use quick_xml::events::Event;

use serde::{Serialize, Deserialize};

use structopt::StructOpt;

#[derive(Debug, StructOpt)]
enum Command {
    #[structopt(name = "tree", about = "Iterate over a tree and find parse all SVGS")]
    Tree {
        #[structopt(parse(from_os_str))]
        tree: PathBuf,
    },
    #[structopt(name = "json-file", about="Parse a JSON file of urs, layout of SVGS")]
    Json {
        #[structopt(parse(from_os_str))]
        file: PathBuf
    },
}

#[derive(StructOpt)]
#[structopt(about = "Tool to parse a tree of SVG files and do color counts")]
struct Opt {
    #[structopt(subcommand)]
    cmd: Command
}

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

#[derive(Serialize, Deserialize, Debug)]
struct JsonEntry {
    urs: String,
    svg: String,
}

fn is_valid_letter(text: String) -> bool {
    match text.as_ref() {
        "A" | "C" | "G" | "U" | "X" => true,
        _ => false,
    }
}

fn count_reader<B: BufRead>(urs: String, reader: &mut Reader<B>) -> Result<Counts, Box<dyn Error>> {
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
            Ok(Event::Start(ref e)) => {
                match e.name() {
                    b"text" => {
                        let text = reader.read_text(e.name(), &mut Vec::new())?;
                        let class = e.attributes().with_checks(false).find(|attr| {
                            match attr {
                                Ok(a) => a.key == b"class",
                                _ => false,
                            }
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
                    },
                    _ => (),
                }
            },
            Err(e) => panic!("Error at position {}: {:?}", reader.buffer_position(), e),
            Ok(Event::Eof) => break,
            _ => (),
        }
    }
    counts.total = counts.changed +
        counts.unchanged +
        counts.inserted +
        counts.moved +
        counts.rotated;
    return Ok(counts);
}

fn count_tree(path: PathBuf) -> Result<(), Box<dyn Error>> {
    let walker = WalkDir::new(path);
    let glob = Glob::new("*.svg")?.compile_matcher();
    let counts = walker.into_iter()
        .filter_map(Result::ok)
        .filter(move |e| glob.is_match(e.file_name()))
        .map(|path| {
            let filename = Path::new(path.file_name());
            let urs = filename.file_stem().unwrap().to_str().unwrap().to_string();
            let mut reader = Reader::from_file(path.path())?;
            return count_reader(urs, &mut reader);
        });
    let mut wtr = csv::Writer::from_writer(io::stdout());
    for count in counts {
        let c = count?;
        wtr.serialize(c)?;
    }
    return Ok(());
}

fn count_json(filename: PathBuf) -> Result<(), Box<dyn Error>> {
    let file = File::open(filename)?;
    let file = BufReader::new(file);
    let counts = file.lines()
        .into_iter()
        .map(|line|  {
            let line = line?.replace("\\\\", "\\");
            let entry: JsonEntry = serde_json::from_str(&line)?;
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

pub fn main() -> Result<(), Box<dyn Error>> {
    let opt = Opt::from_args();
    return match opt.cmd {
        Command::Tree { tree } => count_tree(tree),
        Command::Json { file } => count_json(file),
    };
}
