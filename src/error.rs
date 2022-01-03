use thiserror::Error;

#[derive(Error, Debug)]
pub enum MyError {
    #[error("Git repo does not exist.")]
    GitRepoDoesNotExist,
    #[error("bad path")]
    BadPath,
    #[error("unknown file path")]
    UnknownFilePath,
    #[error("io error")]
    Io {
        #[from]
        source: std::io::Error,
    },
    #[error("http error")]
    Http {
        #[from]
        source: hyper::http::Error,
    },
    #[error("template error")]
    TemplateError {
        #[from]
        source: askama::Error,
    },
    #[error("config file error")]
    ConfigFileError {
        #[from]
        source: toml::de::Error,
    }
}