use crate::Mutant;
use csv::Writer;
use std::error;
use std::fs;
use std::path::{Path, PathBuf};

/// This struct is responsible for logging and exporting mutants
pub struct MutantWriter {
    /// The output directory to write mutants to
    outdir: PathBuf,

    /// Should mutants be logged to outdir/mutants.log?
    log_mutants: bool,

    /// Should mutant sources be written to disk?
    export_mutants: bool,
}

impl MutantWriter {
    pub fn new(outdir: String, log_mutants: bool, export_mutants: bool) -> MutantWriter {
        MutantWriter {
            outdir: PathBuf::from(outdir),
            log_mutants,
            export_mutants,
        }
    }

    /// Write and log mutants based on `self`'s parameters
    pub fn write_mutants(&self, mutants: &[Mutant]) -> Result<(), Box<dyn error::Error>> {
        let mutants_dir = self.outdir.join("mutants");

        if mutants_dir.is_file() {
            fs::remove_file(mutants_dir.clone())?;
        } else if mutants_dir.is_dir() {
            fs::remove_dir_all(mutants_dir.clone())?;
        }

        if self.export_mutants {
            for (i, mutant) in mutants.iter().enumerate() {
                let mid = i + 1;
                let this_mutant_dir = &mutants_dir.join(Path::new(&mid.to_string()));
                Self::write_mutant_to_disk(this_mutant_dir, mutant)?;
            }
        }

        // Log format:
        // 1. Mutant ID
        // 2. Operator
        // 3. File
        // 4. line:column
        // 5. Initial
        // 6. To

        if self.log_mutants {
            let mutants_log = self.outdir.join("mutants.log");
            let mut w = Writer::from_path(mutants_log)?;

            for (i, mutant) in mutants.iter().enumerate() {
                let mid = i + 1;
                let (lineno, colno) = mutant.get_line_column()?;
                let line_col = format!("{}:{}", lineno, colno);
                w.write_record(&[
                    mid.to_string().as_str(),
                    mutant.op.to_string().as_str(),
                    mutant.source.filename().to_str().unwrap(),
                    line_col.as_str(),
                    mutant.orig.as_str(),
                    mutant.repl.as_str(),
                ])?;
            }
        }
        Ok(())
    }

    /// A helper function to write a mutant to disk in the specified directory.
    /// Return the path to the mutant
    pub fn write_mutant_to_disk(
        dir: &Path,
        mutant: &Mutant,
    ) -> Result<PathBuf, Box<dyn error::Error>> {
        fs::create_dir_all(dir)?;
        let filename = mutant.source.filename().file_name().unwrap();
        let filename = dir.join(filename);

        let mutant_contents = mutant.as_source_file()?;
        fs::write(filename.as_path(), mutant_contents)?;
        Ok(filename)
    }
}

#[derive(Debug)]
pub enum MutantWriterError {
    NoSuchDirectory(String),
    NoSuchFile(String),
    InvalidMutantDirFormat(String),
}

impl std::fmt::Display for MutantWriterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MutantWriterError::NoSuchDirectory(msg) => write!(f, "NoSuchDirectory: {}", msg),
            MutantWriterError::NoSuchFile(msg) => write!(f, "NoSuchFile: {}", msg),
            MutantWriterError::InvalidMutantDirFormat(dirname) => write!(
                f,
                "Invalid Mutant Directory: Expected a numeric name but found {}",
                dirname
            ),
        }
    }
}

impl std::error::Error for MutantWriterError {}
