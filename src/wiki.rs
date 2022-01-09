use std::sync::{Mutex, Arc};

use crate::repository::Repository;
use crate::settings::Settings;

/// Wiki god object.
struct WikiInner
{
    settings: Settings,
    repository: Box<dyn Repository + Send>,
}

// TODO: there must be a way to share immutable state that does not involve a mutex
#[derive(Clone)]
pub struct Wiki(Arc<Mutex<WikiInner>>);

impl Wiki
{
    pub fn new(settings: Settings, repository: Box<dyn Repository + Send>) -> Self {
        let inner = WikiInner {
            settings,
            repository,
        };
        Wiki(Arc::from(Mutex::new(inner)))
    }

    pub fn settings(&self) -> Settings {
        // TODO: stop the cloning madness
        self.0.lock().unwrap().settings.clone()
    }

}