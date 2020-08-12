extern crate serde;
extern crate serde_json;
extern crate serde_xml_rs;

use std::collections::HashSet;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::iter::FromIterator;
use std::option::Option;
use std::path::PathBuf;
use std::{thread, time};

use log::{info, warn};

use serde::{Deserialize, Serialize};

use anyhow::Result;

use crate::ena::{species, EnaTaxonInfo};

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
pub enum Rank {
    Species,
    Genus,
    Family,
    Order,
    Class,
    Phylum,
    Kingdom,
    Superkingdom,
    Root,
}

impl Rank {
    pub fn from_string(rank: &String) -> Option<Self> {
        return match rank.as_ref() {
            "species" => Some(Self::Species),
            "genus" => Some(Self::Genus),
            "family" => Some(Self::Family),
            "order" => Some(Self::Order),
            "class" => Some(Self::Class),
            "phylum" => Some(Self::Phylum),
            "kingdom" => Some(Self::Kingdom),
            "superkingdom" => Some(Self::Superkingdom),
            "root" => Some(Self::Root),
            _ => None,
        };
    }

    pub fn ascending() -> Vec<Self> {
        return vec![
            Rank::Species,
            Rank::Genus,
            Rank::Family,
            Rank::Order,
            Rank::Class,
            Rank::Phylum,
            Rank::Kingdom,
            Rank::Superkingdom,
            Rank::Root,
        ];
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct TaxonInfo {
    pub name: String,
    pub taxid: usize,
    pub rank: Rank,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct Mapping {
    pub taxid: usize,
    pub name: Option<String>,
    pub species: Option<TaxonInfo>,
    pub genus: Option<TaxonInfo>,
    pub family: Option<TaxonInfo>,
    pub order: Option<TaxonInfo>,
    pub class: Option<TaxonInfo>,
    pub phylum: Option<TaxonInfo>,
    pub kingdom: Option<TaxonInfo>,
    pub superkingdom: Option<TaxonInfo>,
    pub root: Option<TaxonInfo>,
}

impl Mapping {
    pub fn taxon_at(&self, rank: &Rank) -> Option<&TaxonInfo> {
        match rank {
            Rank::Species => self.species.as_ref(),
            Rank::Genus => self.genus.as_ref(),
            Rank::Family => self.family.as_ref(),
            Rank::Order => self.order.as_ref(),
            Rank::Class => self.class.as_ref(),
            Rank::Phylum => self.phylum.as_ref(),
            Rank::Kingdom => self.kingdom.as_ref(),
            Rank::Superkingdom => self.superkingdom.as_ref(),
            Rank::Root => self.root.as_ref(),
        }
    }

    pub fn is_empty(&self) -> bool {
        if self.taxid == 131567 {
            return false;
        }
        return self.species.is_none()
            && self.genus.is_none()
            && self.family.is_none()
            && self.order.is_none()
            && self.class.is_none()
            && self.phylum.is_none()
            && self.kingdom.is_none()
            && self.superkingdom.is_none();
    }
}

#[derive(Debug)]
struct Report {
    total: usize,
    mapped: usize,
    unmapped: usize,
}

fn lineage_mapping(taxon: &EnaTaxonInfo) -> Mapping {
    let mut mapping = Mapping {
        name: Some(taxon.name.clone()),
        taxid: taxon.taxid,
        species: None,
        genus: None,
        family: None,
        order: None,
        class: None,
        phylum: None,
        kingdom: None,
        superkingdom: None,
        root: Some(TaxonInfo {
            name: String::from("cellular organisms"),
            taxid: 131567,
            rank: Rank::Root,
        }),
    };

    if taxon.rank == Some(String::from("species")) {
        mapping.species = Some(TaxonInfo {
            name: taxon.name.clone(),
            taxid: taxon.taxid,
            rank: Rank::Species,
        })
    }

    for parent in taxon.parent_taxons() {
        let rank: Option<Rank> = parent.rank.as_ref().and_then(|r| Rank::from_string(&r));

        if rank.is_none() {
            continue;
        }

        let rank = rank.unwrap();
        let tinfo = TaxonInfo {
            name: parent.name.to_string(),
            taxid: parent.taxid,
            rank: rank.clone(),
        };

        match rank {
            Rank::Species => mapping.species = Some(tinfo),
            Rank::Genus => mapping.genus = Some(tinfo),
            Rank::Family => mapping.family = Some(tinfo),
            Rank::Order => mapping.order = Some(tinfo),
            Rank::Class => mapping.class = Some(tinfo),
            Rank::Phylum => mapping.phylum = Some(tinfo),
            Rank::Kingdom => mapping.kingdom = Some(tinfo),
            Rank::Superkingdom => mapping.superkingdom = Some(tinfo),
            Rank::Root => mapping.root = Some(tinfo),
        }
    }

    return mapping;
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
        name: None,
        species: None,
        genus: None,
        family: None,
        order: None,
        class: None,
        phylum: None,
        kingdom: None,
        superkingdom: None,
        root: None,
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
            match mapping.is_empty() {
                true => {
                    report.unmapped += 1;
                    warn!("No mapping found for {}", mapping.taxid);
                }
                false => {
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
