use std::sync::Arc;

use crate::error::MyError;
use crate::repository::Repository;
use crate::settings::Settings;

/// Wiki god object.
struct WikiInner {
    settings: Settings,
    repository: Box<dyn Repository + Send + Sync>,
}

// TODO: is there are away to share immutable global without the reference counting? A 'static lifetime somehow?
#[derive(Clone)]
pub struct Wiki(Arc<WikiInner>);

impl Wiki {
    pub fn new(settings: Settings, repository: Box<dyn Repository + Send + Sync>) -> Self {
        let inner = WikiInner {
            settings,
            repository,
        };
        Wiki(Arc::from(inner))
    }

    pub fn settings(&self) -> &Settings {
        &self.0.settings
    }

    pub fn read_file(&self, file_path: &[String]) -> Result<Vec<u8>, MyError> {
        self.0.repository.read_file(file_path)
    }

    pub fn directory_exists(&self, path: &[String]) -> Result<bool, MyError> {
        self.0.repository.directory_exists(path)
    }
}
