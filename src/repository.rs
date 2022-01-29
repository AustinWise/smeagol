use std::{
    io::{Read, Write},
    path::PathBuf,
};

use crate::error::MyError;

//TODO: it is possible to use a borrowed string? Would that reduce copies?
#[derive(Debug, PartialEq, PartialOrd, Eq, Ord)]
pub enum RepositoryItem {
    Directory(String),
    File(String),
}

pub trait Repository: std::fmt::Debug {
    fn read_file(&self, file_path: &[&str]) -> Result<Vec<u8>, MyError>;
    fn write_file(&self, file_path: &[&str], content: &str) -> Result<(), MyError>;
    fn directory_exists(&self, path: &[&str]) -> Result<bool, MyError>;
    fn enumerate_files(&self, directory: &[&str]) -> Result<Vec<RepositoryItem>, MyError>;
}

fn path_element_ok(element: &str) -> bool {
    !element.starts_with('.')
}

#[derive(Debug)]
struct FileSystemRepository {
    root_dir: PathBuf,
}

impl FileSystemRepository {
    fn canonicalize_path(&self, relative_path: &[&str]) -> Result<PathBuf, MyError> {
        let mut path = self.root_dir.to_path_buf();
        for part in relative_path {
            if !path_element_ok(part) {
                return Err(MyError::InvalidPath);
            }
            path.push(part);
        }

        // TODO: figure out if there is way to check for path traversal attacks
        //       when creating a new file. std::fs::canonicalize does not work
        //       for non-existent files.
        //       Currently we are relying on rocket to canonicalize the path.
        Ok(path)
    }
}

impl Repository for FileSystemRepository {
    fn read_file(&self, file_path: &[&str]) -> Result<Vec<u8>, MyError> {
        let path = self.canonicalize_path(file_path)?;
        let mut f = std::fs::File::open(path)?;
        let mut buf = Vec::new();
        f.read_to_end(&mut buf)?;
        Ok(buf)
    }

    fn write_file(&self, file_path: &[&str], content: &str) -> Result<(), MyError> {
        let path = self.canonicalize_path(file_path)?;
        let mut f = std::fs::File::create(path)?;
        f.write_all(content.as_bytes())?;
        f.flush()?;
        Ok(())
    }

    // TODO: consider if this should return error for anything
    fn directory_exists(&self, path: &[&str]) -> Result<bool, MyError> {
        match self.canonicalize_path(path) {
            Ok(path) => Ok(path.is_dir()),
            Err(_) => Ok(false),
        }
    }

    fn enumerate_files(&self, directory: &[&str]) -> Result<Vec<RepositoryItem>, MyError> {
        let path = self.canonicalize_path(directory)?;

        // TODO: is there a nicer way to get a String from a file_name()?
        Ok(std::fs::read_dir(path)?
            .filter_map(|maybe_ent| match maybe_ent {
                Err(_) => None,
                Ok(ent) => {
                    let entry_name = ent.file_name();
                    let entry_name = entry_name.to_str().unwrap();
                    if path_element_ok(entry_name) {
                        if ent.path().is_file() {
                            Some(RepositoryItem::File(entry_name.to_owned()))
                        } else if ent.path().is_dir() {
                            Some(RepositoryItem::Directory(entry_name.to_owned()))
                        } else {
                            None
                        }
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
