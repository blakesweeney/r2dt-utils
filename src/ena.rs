use log::info;

use serde::{Deserialize, Serialize};

use serde_xml_rs::from_reader;

use anyhow::Result;

#[derive(Debug, Deserialize, Serialize)]
pub struct EnaLineageTaxon {
    #[serde(rename = "scientificName")]
    pub name: String,

    #[serde(rename = "taxId")]
    pub taxid: usize,
    pub rank: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct EnaLineage {
    #[serde(rename = "taxon")]
    taxons: Vec<EnaLineageTaxon>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct EnaTaxonInfo {
    #[serde(rename = "scientificName")]
    pub name: String,

    #[serde(rename = "taxId")]
    pub taxid: usize,
    pub rank: Option<String>,
    pub lineage: EnaLineage,
}

impl EnaTaxonInfo {
    pub fn parent_taxons(&self) -> &Vec<EnaLineageTaxon> {
        return &self.lineage.taxons;
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct TaxonSet {
    #[serde(rename = "taxon")]
    taxons: Vec<EnaTaxonInfo>,
}

pub fn species(taxids: &[usize]) -> Result<Vec<EnaTaxonInfo>> {
    let string_taxids: Vec<String> = taxids.iter().map(|t| t.to_string()).collect();
    let tids = string_taxids.join(",");
    info!("Fetching species info for {}", tids);
    let mut url = String::from("https://www.ebi.ac.uk/ena/browser/api/xml/");
    url.push_str(&tids);
    let response = reqwest::blocking::get(&url)?;
    let mut info = Vec::new();
    let body = response.text()?;
    if !body.is_empty() {
        let taxon_set: TaxonSet = from_reader(body.as_bytes())?;
        for taxon in taxon_set.taxons {
            info.push(taxon);
        }
    }
    return Ok(info);
}
