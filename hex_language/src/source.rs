use std::{fs, path::PathBuf};
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
        fs::write(&self.path, bytes).map_err(WriteError::FileWriteFailed)
    }

    fn name(&self) -> &str {
        self.path.to_str().expect("Paths must be unicode")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_read() {
        let file = MemoryFile::new("Something".to_owned(), 4);
        assert_eq!(file.read(), Some(vec![0; 4].into_boxed_slice()));
    }

    #[test]
    fn test_memory_write() {
        let mut file = MemoryFile::new("Something".to_owned(), 0);
        let content = vec![1, 2, 3, 4].into_boxed_slice();
        file.write(&content).unwrap();
        assert_eq!(file.read(), Some(content));
    }
}
