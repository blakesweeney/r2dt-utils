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

use crate::ena::{species, EnaTaxonInfo};

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct TaxonInfo {
    pub name: String,
    pub taxid: usize,
    pub rank: Option<String>,
}

pub type Lineage = Vec<TaxonInfo>;

impl TaxonInfo {
    fn from_ena_taxon(taxon: &EnaTaxonInfo) -> Self {
        return Self {
            name: taxon.name.to_string(),
            taxid: taxon.taxid,
            rank: taxon.rank.as_ref().map(String::to_string),
        };
    }

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

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct Mapping {
    pub taxid: usize,
    pub lineage: Option<Lineage>,
}

#[derive(Debug)]
struct Report {
    total: usize,
    mapped: usize,
    unmapped: usize,
}

fn lineage_mapping(taxon: &EnaTaxonInfo) -> Mapping {
    let mut lineage = Vec::new();
    lineage.push(TaxonInfo::from_ena_taxon(&taxon));
    for parent in taxon.parent_taxons() {
        let tinfo = TaxonInfo {
            name: parent.name.to_string(),
            taxid: parent.taxid,
            rank: parent.rank.as_ref().map(|s| s.to_string()),
        };
        if tinfo.is_standard_level() {
            lineage.push(tinfo);
        }
    }
    return Mapping {
        taxid: taxon.taxid,
        lineage: Some(lineage),
    };
}

fn mappings(taxids: &[usize]) -> Result<Vec<Mapping>> {
    let species = species(taxids)?;
    let mut missing: HashSet<usize> = HashSet::from_iter(taxids.iter().cloned());
    let mut mappings = Vec::new();
    for entry in species {
        let info = lineage_mapping(&entry);
        mappings.push(info);
        missing.remove(&entry.taxid);
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
        .map(|l| l.unwrap().trim().parse::<usize>().unwrap());

    let mut report = Report {
        total: 0,
        mapped: 0,
        unmapped: 0,
    };

    for chunk in taxids.collect::<Vec<usize>>().chunks(chunk_size) {
        report.total += chunk.len();
        let mappings = mappings(chunk)?;
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
