use std::fs::create_dir_all;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;
use std::path::PathBuf;

use anyhow::Result;

use crate::fixups::urs_utils;

fn generate_paths(max_urs: &String, target: &PathBuf) -> Result<Vec<PathBuf>> {
    let max = urs_utils::urs_to_index(max_urs)?;
    let paths = (0..max)
        .step_by(256)
        .map(|index| urs_utils::int_to_urs(index))
        .map(|urs| urs_utils::directory_path(&target, &urs))
        .collect::<Vec<PathBuf>>();
    return Ok(paths);
}

pub fn create_tree(max_urs: &String, base: &PathBuf) -> Result<()> {
    for path in generate_paths(&max_urs, &base)? {
        create_dir_all(path)?;
    }
    return Ok(());
}

pub fn paths(urs_filename: PathBuf, base: PathBuf) -> Result<()> {
    let file = File::open(urs_filename)?;
    let file = BufReader::new(file);
    for line in file.lines() {
        let urs = line?.trim().to_string();
        let path = urs_utils::path_for(&base, &urs);
        let str_path = path.into_os_string().into_string().unwrap();
        println!("{}", str_path);
    }
    return Ok(());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_expected_paths() -> Result<()> {
        assert_eq!(
            generate_paths(&String::from("URS0000000002"), &PathBuf::from("foo"))?,
            vec![PathBuf::from("foo/URS/00/00/00/00")],
        );
        assert_eq!(
            generate_paths(&String::from("URS0000000121"), &PathBuf::from("foo"))?,
            vec![
                PathBuf::from("foo/URS/00/00/00/00"),
                PathBuf::from("foo/URS/00/00/00/01")
            ],
        );
        return Ok(());
    }
}
