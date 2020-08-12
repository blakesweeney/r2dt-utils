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
    taxid: usize,
    model_name: String,
    ancestor_rank: lineage::Rank,
}

type TreeInfo = HashMap<usize, lineage::Mapping>;

fn load_taxid_trees(filename: PathBuf) -> Result<TreeInfo> {
    let file = File::open(filename)?;
    let file = BufReader::new(file);

    let mut info: TreeInfo = HashMap::new();
    for line in file.lines() {
        let line = line?;
        let mapping: lineage::Mapping = serde_json::from_str(&line)?;
        info.insert(mapping.taxid, mapping);
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

    println!("{:?}", sequence_lineage);
    println!("{:?}", model_lineage);
    if assignment.sequence_taxid == assignment.model_taxid {
        return Ok(Lca {
            urs: assignment.urs,
            taxid: assignment.sequence_taxid,
            model_name: assignment.model_name,
            ancestor_rank: lineage::Rank::Species,
        });
    }

    let mut ranks = lineage::Rank::ordered();
    ranks.reverse();
    for rank in ranks {
        let sequence_taxon = sequence_lineage.taxon_at(&rank);
        let model_taxon = model_lineage.taxon_at(&rank);
        if sequence_taxon != model_taxon {
            let previous = rank.previous_rank();
            if previous.is_none() {
                panic!("Impossible state");
            }
            let previous = previous.unwrap();
            return Ok(Lca {
                urs: assignment.urs,
                taxid: assignment.sequence_taxid,
                model_name: assignment.model_name,
                ancestor_rank: previous,
            });
        }
    }

    return Err(anyhow!("Failed to find lca"));
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
