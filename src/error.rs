use thiserror::Error;

#[derive(Error, Debug)]
pub enum MyError {
    #[error("Git repo does not exist.")]
    GitRepoDoesNotExist,
    #[error("bad path")]
    BadPath,
    #[error("io error")]
    Io {
        #[from]
        source: std::io::Error,
    },
    #[error("template error")]
    TemplateError {
        #[from]
        source: askama::Error,
    },
    #[error("Failed to read config file.")]
    ConfigReadError { source: std::io::Error },
    #[error("Failed to parse config file.")]
    ConfigParseError {
        #[from]
        source: toml::de::Error,
    },
    #[error("Not valid UTF-8")]
    BadUtf8 {
        #[from]
        source: std::str::Utf8Error,
    },
}
