use std::path::PathBuf;
use structopt::StructOpt;

#[macro_use]
extern crate lazy_static;

use anyhow::Result;

mod coloring;
mod ena;
mod fixups;
mod fs;
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
            short = "m",
            long = "mapping-file",
            about = "A file mapping from old to new URS",
            parse(from_os_str)
        )]
        rename_file: Option<PathBuf>,

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

    /// Will move a JSON file of SVGs into their final locations
    Split {
        #[structopt(
            short = "m",
            long = "mapping-file",
            about = "A file mapping from old to new URS",
            parse(from_os_str)
        )]
        rename_file: Option<PathBuf>,

        #[structopt(
            name = "FILE",
            about = "A filename containing the JSON encoded SVGs to split",
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

    #[structopt(name = "create-tree", about = "Command to generate the final tree")]
    Fs { max_urs: String, base: PathBuf },

    #[structopt(name = "path-to", about = "Command to generate path for URS ids")]
    PathTo {
        #[structopt(name = "FILE", parse(from_os_str))]
        urs_filename: PathBuf,

        #[structopt(
            name = "DIR",
            about = "The directory to put all svgs into",
            parse(from_os_str)
        )]
        target_directory: PathBuf,
    },

    #[structopt(name = "rename-metadata", about = "Parse a CSV and rename the URS ids")]
    RenameMetadata {
        #[structopt(name = "MAPPING", parse(from_os_str))]
        mapping_file: PathBuf,

        #[structopt(name = "FILE", parse(from_os_str))]
        filename: PathBuf,
    },

    TransferData {
        #[structopt(short, long, env = "ONECLIENT_ACCESS_TOKEN")]
        access_token: String,

        #[structopt(short, long, env = "ONECLIENT_PROVIDER_HOST")]
        host: String,

        #[structopt(short, long, env = "ONEDATA_PATH", default_value = "test_data")]
        remote_path: String,

        #[structopt(short, long)]
        use_http: bool,

        #[structopt(name = "FILE", parse(from_os_str))]
        filename: PathBuf,
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
    simplelog::SimpleLogger::init(level, simplelog::Config::default())
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
            rename_file,
        } => results::move_file(filename, target_directory, rename_file),
        Command::Split {
            filename,
            target_directory,
            rename_file,
        } => results::split_file(filename, target_directory, rename_file),
        Command::Fs { max_urs, base } => fs::create_tree(&max_urs, &base),
        Command::PathTo {
            urs_filename,
            target_directory,
        } => fs::paths(urs_filename, target_directory),
        Command::RenameMetadata {
            mapping_file,
            filename,
        } => results::rename_metadata(mapping_file, filename),
        Command::TransferData {
            access_token,
            host,
            remote_path,
            use_http,
            filename,
        } => {
            let options = results::TransferOptions {
                host,
                access_token,
                remote_path,
                use_http,
            };
            return results::transfer_svgs(&filename, options);
        }
    };
}
