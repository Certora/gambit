use crate::Mutant;
use csv::Writer;
use serde_json::Value::Array;
use similar::TextDiff;
use std::error;
use std::fs;
use std::path::{Path, PathBuf};

/// This struct is responsible for logging and exporting mutants
pub struct MutantWriter {
    /// The output directory to write mutants to
    outdir: PathBuf,
}

impl MutantWriter {
    pub fn new(outdir: String) -> MutantWriter {
        MutantWriter {
            outdir: PathBuf::from(outdir),
        }
    }

    /// Write and log mutants based on `self`'s parameters
    pub fn write_mutants(&self, mutants: &[(Mutant, bool)]) -> Result<(), Box<dyn error::Error>> {
        let mutants_dir = self.outdir.join("mutants");

        if mutants_dir.is_file() {
            fs::remove_file(mutants_dir.clone())?;
        } else if mutants_dir.is_dir() {
            fs::remove_dir_all(mutants_dir.clone())?;
        }

        for (i, (mutant, export)) in mutants.iter().enumerate() {
            if *export {
                let mid = i + 1;
                Self::write_mutant_with_id_to_disk(&mutants_dir, mid, mutant)?;
            }
        }

        // Log format:
        // 1. Mutant ID
        // 2. Operator
        // 3. File
        // 4. line:column
        // 5. Initial
        // 6. To

        // LOG MUTANTS
        let mutants_log = self.outdir.join("mutants.log");
        let mut w = Writer::from_path(mutants_log)?;

        for (i, (mutant, _)) in mutants.iter().enumerate() {
            let mid = i + 1;
            let (lineno, colno) = mutant.get_line_column()?;
            let line_col = format!("{}:{}", lineno, colno);
            w.write_record(&[
                mid.to_string().as_str(),
                mutant.op.to_string().as_str(),
                mutant.source.relative_filename()?.to_str().unwrap(),
                line_col.as_str(),
                mutant.orig.as_str(),
                mutant.repl.as_str(),
            ])?;
        }

        let mut diffs: Vec<String> = vec![];
        for (mutant, _) in mutants {
            diffs.push(Self::diff_mutant(mutant)?);
        }

        let gambit_results_json = PathBuf::from(self.outdir.join("gambit_results.json"));
        log::info!(
            "Writing gambit_results.json to {}",
            &gambit_results_json.display()
        );
        let mut json: Vec<serde_json::Value> = Vec::new();
        for (i, ((mutant, _), diff)) in mutants.iter().zip(diffs).enumerate() {
            let mid = i + 1;
            json.push(serde_json::json!({
                "name": Self::get_mutant_filename(&mutants_dir, mid, mutant),
                "description": mutant.op.to_string(),
                "id": mid,
                "diff": diff,
            }));
        }

        let json_string = serde_json::to_string_pretty(&Array(json)).unwrap();
        fs::write(gambit_results_json, json_string)?;
        Ok(())
    }

    /// A helper function to write a mutant to disk in a subdirectory.
    ///
    /// # Arguments
    ///
    /// * `mutants_dir` - the directory where mutants should be written to
    /// * `mutant` - the mutant to write to disk
    ///
    /// This will compute the contents of `mutant` via `Mutant.as_source_file()`
    /// and write it to file in the `mutants_dir/` directory, creating it if
    /// needed.
    ///
    /// This is used for, e.g., validation (before a mutant id is known), or for
    /// other non-export tasks.
    ///
    /// Return the path to the exported mutant file
    pub fn write_mutant_to_disk(
        mutants_dir: &Path,
        mutant: &Mutant,
    ) -> Result<PathBuf, Box<dyn error::Error>> {
        let filename = mutants_dir.join(mutant.source.filename().file_name().unwrap());
        let mutant_contents = mutant.as_source_file()?;

        log::debug!("Writing mutant {:?} to {}", mutant, &filename.display());

        fs::create_dir_all(mutants_dir)?;
        fs::write(filename.as_path(), mutant_contents)?;

        Ok(filename)
    }
    /// A helper function to write a mutant to disk in a subdirectory specified
    /// by the provided mutant id.
    ///
    /// # Arguments
    ///
    /// * `mutants_dir` - the directory where mutants should be written to
    /// * `mid` - the mutant id of this mutant
    /// * `mutant` - the mutant to write to disk
    ///
    /// This will compute the contents of `mutant` via `Mutant.as_source_file()`
    /// and write it to file in the `mutants_dir/mid/` directory, creating it if
    /// needed.
    ///
    /// Return the path to the exported mutant file
    pub fn write_mutant_with_id_to_disk(
        mutants_dir: &Path,
        mid: usize,
        mutant: &Mutant,
    ) -> Result<PathBuf, Box<dyn error::Error>> {
        let filename = Self::get_mutant_filename(mutants_dir, mid, mutant);
        let mutant_contents = mutant.as_source_file()?;

        log::info!(
            "Writing mutant (mid={}) {:?} to {}",
            mid,
            mutant,
            &filename.display()
        );

        fs::create_dir_all(filename.parent().unwrap())?;
        fs::write(filename.as_path(), mutant_contents)?;

        Ok(filename)
    }

    /// Get the filename where a Mutant will be exported to. This depends
    fn get_mutant_filename(mutants_dir: &Path, mid: usize, mutant: &Mutant) -> PathBuf {
        let rel_filename = match mutant.source.relative_filename() {
            Ok(rel_fn) => rel_fn,
            Err(e) => panic!(
                "Error getting relative filename from {:?}\n\nError:{:?}",
                &mutant.source, e
            ),
        };
        mutants_dir
            .join(Path::new(&mid.to_string()))
            .join(rel_filename)
    }

    fn diff_mutant(mutant: &Mutant) -> Result<String, Box<dyn error::Error>> {
        let orig_contents: String = String::from_utf8_lossy(mutant.source.contents()).into();
        let mutant_contents = mutant.as_source_file().unwrap();

        let diff = TextDiff::from_lines(&orig_contents, &mutant_contents)
            .unified_diff()
            .header("original", "mutant")
            .to_string();
        // Do writing here.

        Ok(diff)
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
