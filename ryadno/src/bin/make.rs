use std::{fs, path::{Path, PathBuf}};

use clap::{Parser, Subcommand};
use include_dir::{Dir, DirEntry, File, include_dir};

pub static TEMPLATES_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/src/templates");

fn main() {
    let cli = Cli::parse();
    match &cli.command {
        Commands::PublishTemplates => match publish_templates() {
            Ok(_) => (),
            Err(err) => {
                eprintln!("Error: {err}")
            }
        },
    }
}

#[derive(Clone, Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Clone)]
enum Commands {
    PublishTemplates,
}

fn publish_templates() -> Result<(), String> {
    let dest_path = Path::new("./templates");

    if dest_path.exists() {
        return Err(
            "Templates directory already exists. Skipping to avoid overwriting.".to_string(),
        );
    }

    fs::create_dir_all(dest_path).map_err(|err| err.to_string())?;
    publish_templates_iter_entries(dest_path.to_path_buf(), TEMPLATES_DIR.entries())?;

    Ok(())
}

fn publish_templates_iter_entries(base_path: PathBuf, entries: &[DirEntry<'_>]) -> Result<(), String> {
    for entry in entries {
        match entry {
            DirEntry::File(file) => {
            	fs::write(base_path.join(file.path()), file.contents()).map_err(|err| err.to_string())?;
            }
            DirEntry::Dir(dir) => {
            	fs::create_dir_all(base_path.join(dir.path())).map_err(|err| err.to_string())?;
            	publish_templates_iter_entries(base_path.clone(), &dir.entries())?
            }
        }
    }

    Ok(())
}
