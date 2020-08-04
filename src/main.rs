use std::error::Error;
use std::path::PathBuf;
use structopt::StructOpt;

mod coloring;
// mod fixups;

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
    #[structopt(name = "report", about = "Process a svg tree to find naming/compression/missing issues")]
    Report {
        #[structopt(parse(from_os_str))]
        tree: PathBuf,

        #[structopt(parse(from_os_str))]
        required: PathBuf,
    },
}

#[derive(Debug, StructOpt)]
enum Command {
    #[structopt(name = "coloring", about = "Count colors in a json-file or in a file tree.")]
    Coloring { 
        #[structopt(subcommand)]
        cmd: ColoringCommand,
    },

    #[structopt(name = "fixup", about = "Find fixes needed for the file tree")]
    Fixups {
        #[structopt(subcommand)]
        cmd: FixupCommand,
    },
}

#[derive(Debug, StructOpt)]
#[structopt(about = "Tool to parse a tree of SVG files and do color counts")]
struct Opt {
    #[structopt(subcommand)]
    cmd: Command,
}

pub fn main() -> Result<(), Box<dyn Error>> {
    let opt = Opt::from_args();
    return match opt.cmd {
        Command::Coloring { cmd } => match cmd {
            ColoringCommand::Tree { tree } => coloring::count_tree(tree),
            ColoringCommand::Json { file } => coloring::count_json(file),
        },
        Command::Fixups { cmd } => match cmd {
            FixupCommand::Report { tree: _tree, required: _required } => Ok(()),
            // FixupCommand::Report { tree, required } => fixups::report(tree, required),
        },
    };
}
