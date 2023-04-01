use crate::Mutant;
use std::error;
use std::fs;
use std::path::{Path, PathBuf};

#[allow(dead_code)]
pub struct OutputDirectoryStructure {
    /// Root of the output directory structure
    root: PathBuf,
    /// Path to mutants_log
    mutants_log: PathBuf,
    /// Path to the mutants/ directory
    mutants_dir: PathBuf,
    /// Path to each mutant directory (root/mutants/1/, root/mutants/2/, etc)
    mutants: Vec<PathBuf>,
}

pub struct MutantWriter {
    outdir: PathBuf,
    log_mutants: bool,
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

    pub fn write_mutants(&self, mutants: &[Mutant]) -> Result<(), Box<dyn error::Error>> {
        let mutants_dir = self.outdir.join("mutants");

        if mutants_dir.is_file() {
            fs::remove_file(mutants_dir.clone())?;
        } else if mutants_dir.is_dir() {
            fs::remove_dir_all(mutants_dir.clone())?;
        }

        for (i, mutant) in mutants.iter().enumerate() {
            let mid = i + 1;
            if self.export_mutants {
                let this_mutant_dir = &mutants_dir.join(Path::new(&mid.to_string()));
                fs::create_dir_all(this_mutant_dir)?;
                let filename = mutant.source.filename().file_name().unwrap();
                let filename = this_mutant_dir.join(filename);

                let mutant_contents = mutant.as_source_file()?;
                fs::write(filename, mutant_contents)?;
            }
            if self.log_mutants {
                // todo
            }
        }
        Ok(())
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
