use clap::{command, Parser};
use rusty_wrenches::{cli, commands};
use std::path::Path;

/// This command fixes mistakingly nested directories when a directory is the only child of another directory with the same name
/// For example, if you have a directory structure like this:
/// somedir/somedir
/// somedir/somedir/file.txt
/// somedir/somedir/file2.txt
/// This command will move all the contents from somedir/somedir to somedir and remove the nested directory
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    // Path to the root directory where to start searching for directories to fix
    #[arg(short, long)]
    path: String,

    // When specified it will unnest folder even when it has different name than parent
    #[arg(short, long)]
    skip_name_match: bool,

    // When specified the command will not actually do any changes
    #[arg(short, long)]
    dry_run: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    cli::init_logging(log::LevelFilter::Info);
    commands::fix_nested_directories(Path::new(&args.path), args.skip_name_match, args.dry_run)?;
    Ok(())
}
