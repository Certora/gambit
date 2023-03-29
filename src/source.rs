use crate::{read_source, SolAST};
use std::error;

/// A source file to be mutated, including all of its different representations.
/// Each source file has a single `Source` associated with it, and intermediate
/// results (e.g, the `contents` and the `ast`), are cached here as they are
/// computed during mutation.
pub struct Source {
    filename: String,
    contents: Option<Vec<u8>>,
    ast: Option<SolAST>,
}

impl Source {
    pub fn contents(&mut self) -> Result<&[u8], Box<dyn error::Error>> {
        match self.contents {
            None => {
                let contents = read_source(self.filename)?;
                self.contents = Some(contents);
                Ok(&self.contents)
            }
            Some(contents) => Ok(&contents),
        }
    }
}
