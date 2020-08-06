use std::path::PathBuf;
use structopt::StructOpt;

#[macro_use]
extern crate lazy_static;

use anyhow::Result;

mod coloring;
mod ena;
mod fixups;
mod lca;
mod lineage;
mod results;

#[derive(Debug, StructOpt)]
enum ColoringCommand {
    #[structopt(name = "tree", about = "Iterate over a tree and find parse all SVGS")]
    Tree {
        #[structopt(parse(from_os_str))]
        tree: PathBuf,
    },
    #[structopt(name = "json-file", about = "Parse a JSON file of urs, layout of SVGS")]
    Json {
        #[structopt(parse(from_os_str))]
        file: PathBuf,
    },
}

#[derive(Debug, StructOpt)]
enum FixupCommand {
    #[structopt(
        name = "report",
        about = "Process a svg tree to find naming/compression/missing issues"
    )]
    Report {
        #[structopt(parse(from_os_str))]
        tree: PathBuf,

        #[structopt(parse(from_os_str))]
        required: PathBuf,
    },
}

#[derive(Debug, StructOpt)]
enum Command {
    #[structopt(
        name = "coloring",
        about = "Count colors in a json-file or in a file tree."
    )]
    Coloring {
        #[structopt(subcommand)]
        cmd: ColoringCommand,
    },

    #[structopt(name = "fixup", about = "Find fixes needed for the file tree")]
    Fixups {
        #[structopt(subcommand)]
        cmd: FixupCommand,
    },

    #[structopt(name = "move", about = "Move SVGs into their final path")]
    Move {
        #[structopt(
            name = "FILE",
            about = "A filename containing a list of result directories to take SVGs from",
            parse(from_os_str)
        )]
        filename: PathBuf,

        #[structopt(
            name = "DIR",
            about = "The directory to put all svgs into",
            parse(from_os_str)
        )]
        target_directory: PathBuf,
    },

    #[structopt(
        name = "lineage",
        about = "Commad to fetch the taxid tree for some taxids"
    )]
    Lineage {
        #[structopt(short = "c", long = "chunk-size", default_value = "10")]
        chunk_size: usize,

        #[structopt(name = "FILE", parse(from_os_str))]
        filename: PathBuf,
    },

    #[structopt(name = "lca", about = "Find the LCA between template and sequences")]
    Lca {
        #[structopt(name = "TAXIDS", parse(from_os_str))]
        taxid_filename: PathBuf,

        #[structopt(name = "ASSIGN", parse(from_os_str))]
        assignments_filename: PathBuf,
    },
}

#[derive(Debug, StructOpt)]
#[structopt(about = "Tool to parse a tree of SVG files and do color counts")]
struct Opt {
    #[structopt(short = "v", long = "verbose", parse(from_occurrences))]
    verbose: u32,

    #[structopt(subcommand)]
    cmd: Command,
}

pub fn main() -> Result<()> {
    let opt = Opt::from_args();

    let level = match opt.verbose {
        0 => simplelog::LevelFilter::Warn,
        1 => simplelog::LevelFilter::Info,
        2 => simplelog::LevelFilter::Debug,
        _ => simplelog::LevelFilter::Trace,
    };
    simplelog::TermLogger::init(
        level,
        simplelog::Config::default(),
        simplelog::TerminalMode::Stderr,
    )
    .unwrap_or_else(|_| eprintln!("Failed to create logger, ignore"));

    return match opt.cmd {
        Command::Coloring { cmd } => match cmd {
            ColoringCommand::Tree { tree } => coloring::count_tree(tree),
            ColoringCommand::Json { file } => coloring::count_json(file),
        },
        Command::Fixups { cmd } => match cmd {
            FixupCommand::Report { tree, required } => fixups::write_report(&tree, required),
        },
        Command::Lineage {
            chunk_size,
            filename,
        } => lineage::write_lineage(chunk_size, filename),
        Command::Lca {
            taxid_filename,
            assignments_filename,
        } => lca::write_lca(taxid_filename, assignments_filename),
        Command::Move {
            filename,
            target_directory,
        } => results::move_file(filename, target_directory),
    };
}
