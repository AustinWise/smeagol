use thiserror::Error;

#[derive(Error, Debug)]
pub enum MyError {
    #[error("Bare Git repos are not yet supported.")]
    BareGitRepo,
    #[error("This is not valid Wiki folder: {path}")]
    GitRepoDoesNotExist { path: std::path::PathBuf },
    #[error("Failed to open Git repo: {err}")]
    GitRepoDoesFailedToOpen { err: git2::Error },
    #[error("Error when performing git operation: {source}")]
    GitError {
        #[from]
        source: git2::Error,
    },
    #[error("Path is not valid.")]
    InvalidPath,
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
    ConfigReadError { source: Box<MyError> },
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
    #[error("Search indexer failed in some way")]
    SearchIndex {
        #[from]
        source: tantivy::TantivyError,
    },
    #[error("Failed to parse search query")]
    SearchQueryParsing {
        #[from]
        source: tantivy::query::QueryParserError,
    },
    #[error("Cross-site request forgery detected")]
    CSRF,
}
