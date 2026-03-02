use std::{
    fs,
    path::{Path, PathBuf},
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum WriteError {
    #[error("Couldn't write file")]
    FileWriteFailed(std::io::Error),
}

pub trait Source {
    fn read(&self) -> Option<Box<[u8]>>;
    fn write(&mut self, bytes: &[u8]) -> Result<(), WriteError>;
    fn name(&self) -> &str;
}

pub struct MemoryFile {
    name: String,
    bytes: Box<[u8]>,
}

impl MemoryFile {
    pub fn new(name: String, length: usize) -> Self {
        Self {
            name,
            bytes: vec![0; length].into_boxed_slice(),
        }
    }
}

impl Source for MemoryFile {
    fn read(&self) -> Option<Box<[u8]>> {
        Some(self.bytes.clone())
    }

    fn write(&mut self, bytes: &[u8]) -> Result<(), WriteError> {
        self.bytes = bytes.to_vec().into_boxed_slice();
        Ok(())
    }

    fn name(&self) -> &str {
        &self.name
    }
}

pub struct File {
    path: PathBuf,
}

impl File {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl Source for File {
    fn read(&self) -> Option<Box<[u8]>> {
        fs::read(&self.path).ok().map(|x| x.into_boxed_slice())
    }

    fn write(&mut self, bytes: &[u8]) -> Result<(), WriteError> {
        fs::write(&self.path, bytes).map_err(|err| WriteError::FileWriteFailed(err))
    }

    fn name(&self) -> &str {
        self.path.to_str().expect("Paths must be unicode")
    }
}
