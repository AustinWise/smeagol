use std::{io::Read, path::Path, path::PathBuf};

use crate::error::MyError;

//TODO: it is possible to use a borrowed string? Would that reduce copies?
pub enum RepositoryItem {
    File(String),
    Directory(String),
}

pub trait Repository {
    fn read_file(&self, file_path: &str) -> Result<Vec<u8>, MyError>;
    fn enumerate_files(&self, directory: &str) -> Result<Vec<RepositoryItem>, MyError>;
}

struct FileSystemRepository {
    root_dir: PathBuf,
}

impl FileSystemRepository {
    fn canonicalize_path(&self, relative_path: &str) -> Result<PathBuf, MyError> {
        let mut path = self.root_dir.to_path_buf();
        //TODO: decide whether to always require paths to start with the a slash or to always NOT start with a slash
        if relative_path.starts_with('/') {
            path.push(&relative_path[1..]);
        } else {
            path.push(&relative_path);
        }

        let path = path.canonicalize()?;
        if path.starts_with(&self.root_dir) {
            Ok(path)
        } else {
            //TODO: maybe a more specific error?
            Err(MyError::BadPath)
        }
    }
}

impl Repository for FileSystemRepository {
    fn read_file(&self, file_path: &str) -> Result<Vec<u8>, MyError> {
        let path = self.canonicalize_path(file_path)?;
        let mut f = std::fs::File::open(path)?;
        let mut buf = Vec::new();
        f.read_to_end(&mut buf)?;
        Ok(buf)
    }

    fn enumerate_files(&self, directory: &str) -> Result<Vec<RepositoryItem>, MyError> {
        let path = self.canonicalize_path(directory)?;

        // TODO: is there a nicer way to get a String from a file_name()?
        Ok(std::fs::read_dir(path)?
            .filter_map(|maybe_ent| match maybe_ent {
                Err(_) => None,
                Ok(ent) => {
                    if ent.path().is_file() {
                        Some(RepositoryItem::File(
                            ent.path().file_name().unwrap().to_str().unwrap().to_owned(),
                        ))
                    } else if ent.path().is_dir() {
                        Some(RepositoryItem::Directory(
                            ent.path().file_name().unwrap().to_str().unwrap().to_owned(),
                        ))
                    } else {
                        None
                    }
                }
            })
            .collect())
    }
}

pub fn create_file_system_repository(dir_path: PathBuf) -> Result<impl Repository, MyError> {
    let root_dir = dir_path.canonicalize()?;
    if root_dir.is_dir() {
        Ok(FileSystemRepository { root_dir })
    } else {
        Err(MyError::GitRepoDoesNotExist)
    }
}
