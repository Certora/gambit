use crate::read_source;
use std::{
    error,
    path::{Path, PathBuf},
};

/// A source file, including its name and its contents, to be mutated
#[derive(Debug)]
pub struct Source {
    filename: PathBuf,
    contents: Vec<u8>,
}

impl Source {
    pub fn new(filename: PathBuf) -> Result<Source, Box<dyn error::Error>> {
        let contents = read_source(&filename)?;
        Ok(Source { filename, contents })
    }

    /// Get the filename of this source
    pub fn filename(&self) -> &Path {
        self.filename.as_path()
    }

    /// Get the contents of this source, computing from `filename` if necessary
    pub fn contents(&self) -> &[u8] {
        &self.contents
    }
}
