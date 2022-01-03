use std::fs::canonicalize;
use std::path::PathBuf;

use clap::Parser;

use crate::error::MyError;

#[derive(clap::Parser, Debug, Clone)]
#[clap(about, version, author)]
struct Args {
    /// Path to the directory containing the wiki Git repository.
    #[clap(parse(from_os_str))]
    git_repo: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct Settings {
    git_repo: PathBuf,
}

impl Settings {
    pub fn git_repo(&self) -> &PathBuf {
        &self.git_repo
    }
}

pub fn parse_settings_from_args() -> Result<Settings, MyError> {
    let args = Args::parse();

    let git_repo = if let Some(dir) = args.git_repo {
        dir
    } else {
        std::env::current_dir()?
    };
    let git_repo = canonicalize(git_repo)?;

    let ret = Settings {
        git_repo,
    };
    Ok(ret)
}
