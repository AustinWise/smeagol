use std::sync::Arc;

use crate::error::MyError;
use crate::repository::Repository;
use crate::settings::Settings;

/// Wiki god object.
struct WikiInner {
    settings: Settings,
    repository: Box<dyn Repository + Send + Sync>,
}

// TODO: there must be a way to share immutable state that does not involve a mutex
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

    pub fn settings(&self) -> Settings {
        // TODO: stop the cloning madness
        self.0.settings.clone()
    }

    pub fn read_file(&self, file_path: &str) -> Result<Vec<u8>, MyError> {
        self.0.repository.as_ref().read_file(file_path)
    }
}
