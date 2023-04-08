use std::{
    io::{Read, Write},
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
    sync::Mutex,
};

use bitflags::bitflags;
use git2::ObjectType;

use crate::error::MyError;

bitflags! {
    pub struct RepositoryCapability: u32 {
        const SUPPORTS_EDIT_MESSAGE = 0b00000001;
    }
}

//TODO: it is possible to use a borrowed string? Would that reduce copies?
#[derive(Debug, PartialEq, PartialOrd, Eq, Ord)]
pub enum RepositoryItem {
    Directory(String),
    File(String),
}

pub trait Repository {
    fn capabilities(&self) -> RepositoryCapability;
    fn read_file(&self, file_path: &[&str]) -> Result<Vec<u8>, MyError>;
    fn write_file(&self, file_path: &[&str], message: &str, content: &str) -> Result<(), MyError>;
    fn directory_exists(&self, path: &[&str]) -> Result<bool, MyError>;
    fn file_exists(&self, path: &[&str]) -> Result<bool, MyError>;
    fn enumerate_files(&self, directory: &[&str]) -> Result<Vec<RepositoryItem>, MyError>;
}

pub struct RepoBox(Box<dyn Repository + Sync + Send>);

impl Deref for RepoBox {
    type Target = dyn Repository + Sync + Send;
    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl DerefMut for RepoBox {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.deref_mut()
    }
}

fn path_element_ok(element: &str) -> bool {
    !element.starts_with('.')
}

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
    fn capabilities(&self) -> RepositoryCapability {
        RepositoryCapability::empty()
    }

    fn read_file(&self, file_path: &[&str]) -> Result<Vec<u8>, MyError> {
        let path = self.canonicalize_path(file_path)?;
        let mut f = std::fs::File::open(path)?;
        let mut buf = Vec::new();
        f.read_to_end(&mut buf)?;
        Ok(buf)
    }

    fn write_file(&self, file_path: &[&str], _message: &str, content: &str) -> Result<(), MyError> {
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

    // TODO: consider if this should return error for anything
    fn file_exists(&self, path: &[&str]) -> Result<bool, MyError> {
        match self.canonicalize_path(path) {
            Ok(path) => Ok(path.is_file()),
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

struct GitRepository {
    path: PathBuf,
    repo: Mutex<git2::Repository>,
}

fn get_git_dir<'repo>(
    repo: &'repo std::sync::MutexGuard<git2::Repository>,
    file_paths: &[&str],
) -> Result<git2::Tree<'repo>, MyError> {
    let head = repo.head()?;
    let mut root = head.peel_to_tree()?;
    for path in file_paths {
        let obj = match root.get_name(path) {
            Some(te) => te.to_object(repo)?,
            None => {
                return Err(MyError::InvalidPath);
            }
        };
        root = match obj.as_tree() {
            Some(tree) => tree.to_owned(),
            None => {
                return Err(MyError::InvalidPath);
            }
        };
    }
    Ok(root)
}

impl Repository for GitRepository {
    fn capabilities(&self) -> RepositoryCapability {
        RepositoryCapability::SUPPORTS_EDIT_MESSAGE
    }

    fn read_file(&self, file_path: &[&str]) -> Result<Vec<u8>, MyError> {
        let (filename, file_paths) = match file_path.split_last() {
            Some(tup) => tup,
            None => {
                return Err(MyError::InvalidPath);
            }
        };

        let repo = self.repo.lock().unwrap();
        let root = get_git_dir(&repo, file_paths)?;

        let file_obj = match root.get_name(filename) {
            Some(te) => te.to_object(&repo)?,
            None => {
                return Err(MyError::InvalidPath);
            }
        };

        match file_obj.as_blob() {
            Some(b) => Ok(b.content().to_owned()),
            None => Err(MyError::InvalidPath),
        }
    }

    fn write_file(&self, file_path: &[&str], message: &str, content: &str) -> Result<(), MyError> {
        if file_path.is_empty() {
            return Err(MyError::InvalidPath);
        }

        let file_path = file_path.join("/");

        // Get as many Git objects ready before writing the file.
        let repo = self.repo.lock().unwrap();
        let mut index = repo.index()?;
        let sig = repo.signature()?;
        let head = repo.head()?;
        let head_commit = head.peel_to_commit()?;

        let mut path = self.path.clone();
        path.push(&file_path);
        let path = path.canonicalize()?;
        if !path.starts_with(&self.path) {
            return Err(MyError::InvalidPath);
        }

        // TODO: maybe write the file directly into the index or as a tree.
        // This would support bare Git repos.
        std::fs::write(&path, content)?;

        index.add_path(Path::new(&file_path))?;
        index.write()?;
        let tree = index.write_tree()?;
        let tree = repo.find_tree(tree)?;

        repo.commit(head.name(), &sig, &sig, message, &tree, &[&head_commit])?;

        Ok(())
    }

    fn directory_exists(&self, path: &[&str]) -> Result<bool, MyError> {
        let repo = self.repo.lock().unwrap();
        let ret = get_git_dir(&repo, path).is_ok();
        Ok(ret)
    }

    fn file_exists(&self, path: &[&str]) -> Result<bool, MyError> {
        let (filename, file_paths) = match path.split_last() {
            Some(tup) => tup,
            None => {
                return Err(MyError::InvalidPath);
            }
        };

        let repo = self.repo.lock().unwrap();
        let root = match get_git_dir(&repo, file_paths) {
            Ok(tree) => tree,
            Err(_) => return Ok(false),
        };

        let file_obj = match root.get_name(filename) {
            Some(te) => te.to_object(&repo)?,
            None => {
                return Ok(false);
            }
        };

        Ok(file_obj.as_blob().is_some())
    }

    fn enumerate_files(&self, directory: &[&str]) -> Result<Vec<RepositoryItem>, MyError> {
        let repo = self.repo.lock().unwrap();
        let tree = get_git_dir(&repo, directory)?;
        Ok(tree
            .into_iter()
            .filter_map(|te| match te.kind() {
                Some(ObjectType::Blob) => Some(RepositoryItem::File(te.name().unwrap().to_owned())),
                Some(ObjectType::Tree) => {
                    Some(RepositoryItem::Directory(te.name().unwrap().to_owned()))
                }
                _ => None,
            })
            .collect())
    }
}

pub fn create_git_repository(dir_path: PathBuf) -> Result<RepoBox, MyError> {
    let repo = match git2::Repository::open(&dir_path) {
        Ok(repo) => repo,
        Err(err) => {
            return Err(MyError::GitRepoDoesFailedToOpen { err });
        }
    };
    if repo.is_bare() {
        return Err(MyError::BareGitRepo);
    }
    Ok(RepoBox(Box::new(GitRepository {
        path: dir_path,
        repo: Mutex::new(repo),
    })))
}

pub fn create_repository(use_fs: bool, dir_path: PathBuf) -> Result<RepoBox, MyError> {
    let root_dir = match dir_path.canonicalize() {
        Ok(dir) => dir,
        Err(_) => {
            return Err(MyError::GitRepoDoesNotExist { path: dir_path });
        }
    };
    if !root_dir.is_dir() {
        return Err(MyError::GitRepoDoesNotExist { path: root_dir });
    }
    if use_fs {
        Ok(RepoBox(Box::new(FileSystemRepository { root_dir })))
    } else {
        create_git_repository(root_dir)
    }
}
