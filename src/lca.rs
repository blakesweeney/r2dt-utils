use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::io::BufReader;
use std::path::PathBuf;

use anyhow::{anyhow, Result};

use serde::{Deserialize, Serialize};

use crate::lineage;

#[derive(Debug, Deserialize)]
struct DiagramAssignment {
    urs: String,
    model_name: String,
    sequence_taxid: usize,
    model_taxid: usize,
}

#[derive(Debug, Deserialize, Serialize)]
struct Lca {
    urs: String,
    model_name: String,

    #[serde(flatten)]
    taxon: lineage::TaxonInfo,
}

type TreeInfo = HashMap<usize, lineage::Lineage>;

fn load_taxid_trees(filename: PathBuf) -> Result<TreeInfo> {
    let file = File::open(filename)?;
    let file = BufReader::new(file);

    let mut info: TreeInfo = HashMap::new();
    for line in file.lines() {
        let line = line?;
        let mapping: lineage::Mapping = serde_json::from_str(&line)?;
        match mapping.lineage {
            Some(l) => {
                info.insert(mapping.taxid, l);
                ();
            }
            None => (),
        };
    }
    return Ok(info);
}

fn lca(trees: &TreeInfo, assignment: DiagramAssignment) -> Result<Lca> {
    let sequence_lineage = match trees.get(&assignment.sequence_taxid) {
        Some(v) => Ok(v),
        None => Err(anyhow!(
            "Missing lineage for {}",
            &assignment.sequence_taxid
        )),
    }?;
    let model_lineage = match trees.get(&assignment.model_taxid) {
        Some(v) => Ok(v),
        None => Err(anyhow!("Missing lineage for {}", &assignment.model_taxid)),
    }?;

    if assignment.sequence_taxid == assignment.model_taxid {
        let last: lineage::TaxonInfo = match sequence_lineage.last() {
            Some(l) => lineage::TaxonInfo {
                name: l.name.to_string(),
                taxid: l.taxid,
                rank: l.rank.as_ref().map(|s| s.to_string()),
            },
            None => panic!("Should never have empty lineage"),
        };
        return Ok(Lca {
            urs: assignment.urs,
            model_name: assignment.model_name,
            taxon: last,
        });
    }

    let pairs = sequence_lineage.iter().zip(model_lineage.iter());

    for (seq, temp) in pairs {
        if seq != temp {
            let prev = match sequence_lineage.last() {
                Some(l) => lineage::TaxonInfo {
                    name: l.name.to_string(),
                    taxid: l.taxid,
                    rank: l.rank.as_ref().map(|s| s.to_string()),
                },
                None => panic!("Lineage cannot be empty"),
            };
            return Ok(Lca {
                urs: assignment.urs,
                model_name: assignment.model_name,
                taxon: prev,
            });
        }
    }

    panic!("Impossible state");
}

pub fn write_lca(taxid_filename: PathBuf, assignments_filename: PathBuf) -> Result<()> {
    let mut wtr = csv::Writer::from_writer(io::stdout());
    let trees = load_taxid_trees(taxid_filename)?;
    let file = File::open(assignments_filename)?;
    let file = BufReader::new(file);
    let mut reader = csv::Reader::from_reader(file);

    for result in reader.deserialize() {
        let assignment: DiagramAssignment = result?;
        let lca = lca(&trees, assignment)?;
        wtr.serialize(lca)?;
    }

    return Ok(());
}
