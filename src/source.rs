use crate::{read_source, simplify_path};
use std::{
    error, fmt,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub enum SourceError {
    /// Indicate an out of bound position in a source file
    PositionOutOfBoundsError(usize, String),
    /// Indicate that we couldn't find a line/column number at a given position for a source file
    LineColumnLookupError(usize, String),
}

impl fmt::Display for SourceError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "SourceError")
    }
}

impl error::Error for SourceError {}

/// A source file, including its name and its contents, to be mutated
pub struct Source {
    filename: PathBuf,
    contents: Vec<u8>,
    newlines: Vec<usize>,
}

impl std::fmt::Debug for Source {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        f.debug_struct("Source")
            .field("filename", &self.filename)
            .field("contents", &String::from("[...]"))
            .field("newlines", &String::from("[...]"))
            .finish()
    }
}

impl Source {
    pub fn new(filename: PathBuf) -> Result<Source, Box<dyn error::Error>> {
        let filename = simplify_path(&filename)?;
        let contents = read_source(&filename)?;
        let newlines: Vec<usize> = contents
            .iter()
            .enumerate()
            .filter(|(_, c)| **c == b'\n')
            .map(|(i, _)| i + 2)
            .collect();

        Ok(Source {
            filename,
            contents,
            newlines,
        })
    }

    /// Get the filename of this source
    pub fn filename(&self) -> &Path {
        self.filename.as_path()
    }

    pub fn filename_as_str(&self) -> String {
        self.filename.to_str().unwrap().into()
    }

    /// Get the contents of this source, computing from `filename` if necessary
    pub fn contents(&self) -> &[u8] {
        &self.contents
    }

    /// Get a (line, column) pair that represents which line and column this
    /// mutant occurs at. Lines and columns are both 1-indexed.
    pub fn get_line_column(&self, pos: usize) -> Result<(usize, usize), Box<dyn error::Error>> {
        if pos >= self.contents.len() {
            return Err(Box::new(SourceError::PositionOutOfBoundsError(
                pos,
                self.filename_as_str(),
            )));
        }

        let newlines = &self.newlines;
        if let Some((lineno, nlpos)) = newlines
            .iter()
            .enumerate()
            .rev()
            .find(|(_, nlpos)| nlpos < &&pos)
        {
            let columnno = pos - nlpos + 1;
            Ok((lineno + 2, columnno))
        } else if &pos < newlines.get(0).unwrap() {
            Ok((1, pos + 1))
        } else {
            Err(Box::new(SourceError::LineColumnLookupError(
                pos,
                self.filename_as_str(),
            )))
        }
    }
}
