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
            let (lineno, colno) = mutant.get_line_column();
            let line_col = format!("{}:{}", lineno + 1, colno + 1);
            w.write_record([
                mid.to_string().as_str(),
                mutant.op.short_name().as_str(),
                mutant.vfs_path().to_str().unwrap(),
                line_col.as_str(),
                mutant.orig.as_str(),
                mutant.repl.as_str(),
            ])?;
        }

        let mut diffs: Vec<String> = vec![];
        for (mutant, _) in mutants {
            diffs.push(Self::diff_mutant(mutant)?);
        }

        let gambit_results_json = self.outdir.join("gambit_results.json");
        log::info!(
            "Writing gambit_results.json to {}",
            &gambit_results_json.display()
        );
        let mut json: Vec<serde_json::Value> = Vec::new();
        for (i, ((mutant, _), diff)) in mutants.iter().zip(diffs).enumerate() {
            let mid = i + 1;
            json.push(serde_json::json!({
                "name": Self::get_mutant_filename(&PathBuf::from("mutants"), mid, mutant),
                "description": mutant.op.to_string(),
                "op": mutant.op.short_name(),
                "id": mid.to_string(),
                "diff": diff,
                "original": mutant.vfs_path(),
                "orig": &mutant.orig,
                "repl": &mutant.repl,
                "line": &mutant.get_line_column().0 + 1,
                "col": &mutant.get_line_column().1 + 1
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
    pub fn write_mutant_to_dir(
        mutants_dir: &Path,
        mutant: &Mutant,
    ) -> Result<PathBuf, Box<dyn error::Error>> {
        let filename = mutants_dir.join(mutant.vfs_path());
        let mutant_contents = mutant.mutant_source()?;

        log::debug!("Writing mutant {:?} to {}", mutant, &filename.display());

        fs::create_dir_all(mutants_dir)?;
        fs::write(filename.as_path(), mutant_contents)?;

        Ok(filename)
    }

    /// Write a mutant's content to a particular file
    ///
    /// # Arguments
    ///
    /// * `filename` - a path to the file that will be written with the mutant's content
    /// * `mutant` - the mutant to be written
    pub fn write_mutant_to_file(
        filename: &Path,
        mutant: &Mutant,
    ) -> Result<(), Box<dyn error::Error>> {
        let mutant_contents = mutant.mutant_source()?;

        log::debug!("Writing mutant {:?} to {}", mutant, &filename.display());

        fs::write(filename, mutant_contents)?;

        Ok(())
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
        let mutant_contents = mutant.mutant_source()?;

        log::debug!(
            "Writing mutant (mid={}) {:?} to {}",
            mid,
            mutant,
            &filename.display()
        );

        fs::create_dir_all(filename.parent().unwrap())?;
        fs::write(filename.as_path(), mutant_contents)?;

        Ok(filename)
    }

    /// Get the filename where a Mutant will be exported to.
    ///
    /// This is computed from the relative path of the original sourcefile, relative to
    /// the specified `sourceroot`, and is computed with `Source.relative_filename()`
    fn get_mutant_filename(mutants_dir: &Path, mid: usize, mutant: &Mutant) -> PathBuf {
        let rel_filename = mutant.vfs_path();
        mutants_dir
            .join(Path::new(&mid.to_string()))
            .join(rel_filename)
    }

    /// Get the diff of the mutant and the original file
    fn diff_mutant(mutant: &Mutant) -> Result<String, Box<dyn error::Error>> {
        let orig_contents = mutant.original_source().to_string();
        let mutant_contents = mutant.mutant_source().unwrap();

        let diff = TextDiff::from_lines(&orig_contents, &mutant_contents)
            .unified_diff()
            .header("original", "mutant")
            .to_string();

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
