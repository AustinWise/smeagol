use std::fs::canonicalize;
use std::sync::{Arc, Mutex};
use std::path::PathBuf;

use log::info;

use clap::Parser;


#[derive(clap::Parser, Debug, Clone)]
#[clap(about, version, author)]
struct ArgsImp {
    /// Name of the person to greet
    #[clap(parse(from_os_str))]
    git_repo: PathBuf,
}

// TODO: figure out if we really need this arc and mutex stuff
#[derive(Debug, Clone)]
pub struct Args(Arc<Mutex<ArgsImp>>);

impl Args {
    pub fn parse() -> Args {
        let mut args = ArgsImp::parse();
        args.git_repo = canonicalize(args.git_repo)
            .expect("Git repo does not exist, check the path provided.");
        info!("canonicalize git repo dir: {:?}", args.git_repo);
        Args(Arc::from(Mutex::from(args)))
    }

    pub fn git_repo(&self) -> PathBuf {
        self.0.lock().unwrap().git_repo.clone()
    }
}