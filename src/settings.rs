use std::default::Default;
use std::net::{IpAddr, Ipv4Addr};
use std::path::PathBuf;

use serde::Deserialize;

use crate::error::MyError;
use crate::repository::{RepoBox, RepositoryItem};

#[derive(clap::Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Path to the directory containing the wiki Git repository.
    git_repo: Option<PathBuf>,
    /// The IP address to bind to. Defaults to 127.0.0.1
    #[arg(long)]
    host: Option<IpAddr>,
    /// The TCP Port to bind to. Defaults to 8000
    #[arg(long)]
    port: Option<u16>,
    /// Use the file system to read the wiki, not Git.
    #[arg(long)]
    fs: bool,
}

impl Args {
    pub fn git_repo(&self) -> Option<PathBuf> {
        self.git_repo.clone()
    }

    pub fn use_fs(&self) -> bool {
        self.fs
    }
}

#[derive(Default, Deserialize)]
struct Config {
    /// The name of the index page. "Home" by default.
    #[serde(rename = "index-page")]
    index_page: Option<String>,
    /// Whether the first H1 should become the title of a page.
    #[serde(rename = "h1-title")]
    h1_title: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct Settings {
    index_page: String,
    h1_title: bool,
    host: IpAddr,
    port: u16,
}

impl Settings {
    #[cfg(test)]
    pub(crate) fn new(index_page: &str, h1_title: bool) -> Settings {
        Settings {
            index_page: index_page.to_owned(),
            h1_title,
            host: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            port: 8000,
        }
    }

    pub fn index_page(&self) -> &str {
        &self.index_page
    }

    pub fn h1_title(&self) -> bool {
        self.h1_title
    }

    pub fn host(&self) -> IpAddr {
        self.host
    }

    pub fn port(&self) -> u16 {
        self.port
    }
}

fn load_config(repo: &RepoBox) -> Result<Config, MyError> {
    const CONFIG_FILE_NAME: &str = "smeagol.toml";

    if !repo.enumerate_files(&[])?.iter().any(|f| match f {
        RepositoryItem::File(name) => name == CONFIG_FILE_NAME,
        _ => false,
    }) {
        return Ok(Default::default());
    };

    let file_contents = match repo.read_file(&[CONFIG_FILE_NAME]) {
        Ok(bytes) => bytes,
        Err(err) => {
            return Err(MyError::ConfigReadError {
                source: Box::new(err),
            })
        }
    };
    let config_str = std::str::from_utf8(&file_contents)?;

    Ok(toml::from_str(config_str)?)
}

pub fn parse_settings_from_args(args: Args, repo: &RepoBox) -> Result<Settings, MyError> {
    let config = load_config(repo)?;

    let ret = Settings {
        index_page: config.index_page.unwrap_or_else(|| "README".into()),
        h1_title: config.h1_title.unwrap_or(false),
        host: args
            .host
            .unwrap_or_else(|| IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))),
        port: args.port.unwrap_or(8000),
    };
    Ok(ret)
}
