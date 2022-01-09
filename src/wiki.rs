use crate::repository::Repository;
use crate::settings::Settings;

/// Wiki god object.
pub struct Wiki<TRepository>
where
    TRepository: Repository,
{
    settings: Settings,
    repository: TRepository,
}

impl<TRepository> Wiki<TRepository>
where
    TRepository: Repository,
{
    pub fn new(settings: Settings, repository: TRepository) -> Self {
        Wiki {
            settings,
            repository,
        }
    }

    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    pub fn repository(&self) -> &TRepository {
        &self.repository
    }
}