extern crate serde;
extern crate serde_json;
extern crate serde_xml_rs;

use std::collections::HashSet;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::iter::FromIterator;
use std::path::PathBuf;
use std::{thread, time};

use log::{info, warn};

use serde::{Deserialize, Serialize};

use anyhow::Result;

use serde_xml_rs::from_reader;

#[derive(Debug, Deserialize, Serialize)]
struct LineageTaxon {
    #[serde(rename = "scientificName")]
    name: String,

    #[serde(rename = "taxId")]
    taxid: i64,
    rank: Option<String>,
}

impl LineageTaxon {
    fn is_standard_level(&self) -> bool {
        match &self.rank {
            None => false,
            Some(r) => match r.as_ref() {
                "species" => true,
                "genus" => true,
                "family" => true,
                "class" => true,
                "order" => true,
                "phylum" => true,
                "kingdom" => true,
                "superkingdom" => true,
                _ => false,
            },
        }
    }
}

#[derive(Debug, Serialize)]
struct Mapping {
    taxid: i64,
    lineage: Option<Vec<LineageTaxon>>,
}

#[derive(Debug, Deserialize)]
struct Lineage {
    #[serde(rename = "taxon")]
    taxons: Vec<LineageTaxon>,
}

#[derive(Debug, Deserialize)]
struct Taxon {
    #[serde(rename = "scientificName")]
    name: String,

    #[serde(rename = "taxId")]
    taxid: i64,
    rank: Option<String>,
    lineage: Lineage,
}

#[derive(Debug, Deserialize)]
struct TaxonSet {
    #[serde(rename = "taxon")]
    taxons: Vec<Taxon>,
}

#[derive(Debug)]
struct Report {
    total: usize,
    mapped: usize,
    unmapped: usize,
}

fn lineage_mapping(taxon: Taxon) -> Mapping {
    let mut lineage = Vec::new();
    for subtaxon in taxon.lineage.taxons {
        if subtaxon.is_standard_level() {
            lineage.push(subtaxon);
        }
    }
    let lineage = match lineage.is_empty() {
        true => None,
        false => Some(lineage),
    };
    return Mapping {
        taxid: taxon.taxid,
        lineage: lineage,
    };
}

fn species(taxids: &[i64]) -> Result<Vec<Mapping>> {
    let string_taxids: Vec<String> = taxids.iter().map(|t| t.to_string()).collect();
    let tids = string_taxids.join(",");
    info!("Fetching mapping for {}", tids);
    let mut url = String::from("https://www.ebi.ac.uk/ena/browser/api/xml/");
    url.push_str(&tids);
    let response = reqwest::blocking::get(&url)?;
    let mut mappings = Vec::new();
    let mut missing: HashSet<i64> = HashSet::from_iter(taxids.iter().cloned());
    let body = response.text()?;
    if !body.is_empty() {
        let taxon_set: TaxonSet = from_reader(body.as_bytes())?;
        for taxon in taxon_set.taxons {
            missing.remove(&taxon.taxid);
            let mapping = lineage_mapping(taxon);
            mappings.push(mapping);
        }
    }
    let extra = missing.iter().map(|taxid| Mapping {
        taxid: *taxid,
        lineage: None,
    });
    mappings.extend(extra);
    return Ok(mappings);
}

pub fn write_lineage(chunk_size: usize, filename: PathBuf) -> Result<()> {
    let file = File::open(filename)?;
    let reader = BufReader::new(file);

    let taxids = reader
        .lines()
        .into_iter()
        .map(|l| l.unwrap().trim().parse::<i64>().unwrap());

    let mut report = Report {
        total: 0,
        mapped: 0,
        unmapped: 0,
    };
    for chunk in taxids.collect::<Vec<i64>>().chunks(chunk_size) {
        report.total += chunk.len();
        let mappings = species(chunk)?;
        for mapping in mappings {
            match mapping.lineage {
                None => {
                    report.unmapped += 1;
                    warn!("No species taxid found for {}", mapping.taxid);
                }
                Some(_) => {
                    report.mapped += 1;
                    println!("{}", serde_json::to_string(&mapping)?);
                }
            }
        }
        let dur = time::Duration::from_millis(200);
        thread::sleep(dur)
    }
    info!("Status: {:?}", report);
    return Ok(());
}
