// #[macro_use]
// extern crate structopt;

extern crate glob;

use std::io;
use std::path::Path;
use std::error::Error;
use std::iter::Iterator;

use glob::glob;

use svg::parser::Event;

use serde::{Serialize, Deserialize};

use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(about = "Tool to parse a tree of SVG files and do color counts")]
struct Opt {
    /// Input file
    input: String,
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

fn count(path: &Path) -> Result<Counts, Box<dyn Error>> {
    let urs = path.file_stem().unwrap().to_str().unwrap().to_string();
    let mut counts = Counts { urs, changed: 0, unchanged: 0, inserted: 0, moved: 0, rotated: 0, total: 0 };
    let doc = svg::open(path)?;

    for event in doc {
        match event {
            Event::Tag("text", _, attributes) => {
                let class = attributes.get("class");
                match class {
                    None => (),
                    Some(v) => match v.as_ref() {
                        "green" => counts.changed += 1,
                        "black" => counts.unchanged += 1,
                        "red" => counts.inserted += 1,
                        "blue" => counts.moved += 1,
                        "brown" => counts.rotated += 1,
                        _ => (),
                    }
                }
            },
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

pub fn main() -> Result<(), Box<dyn Error>> {
    let opt = Opt::from_args();

    let mut wtr = csv::Writer::from_writer(io::stdout());

    let counts = glob(&opt.input)?
        .filter_map(Result::ok)
        .map(|p| count(&p));

    for count in counts {
        let c = count?;
        wtr.serialize(c)?;
    }
    return Ok(());
}
