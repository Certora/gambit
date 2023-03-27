use rand::{seq::IteratorRandom, thread_rng};
use std::fs;
use std::path::{Path, PathBuf};

/// This module downsamples mutants.
use std::error;

#[derive(Debug)]
pub enum MutantFilterError {
    NoSuchDirectory(String),
    NoSuchFile(String),
    InvalidMutantDirFormat(String),
}

impl std::fmt::Display for MutantFilterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MutantFilterError::NoSuchDirectory(msg) => write!(f, "NoSuchDirectory: {}", msg),
            MutantFilterError::NoSuchFile(msg) => write!(f, "NoSuchFile: {}", msg),
            MutantFilterError::InvalidMutantDirFormat(dirname) => write!(
                f,
                "Invalid Mutant Directory: Expected a numeric name but found {}",
                dirname
            ),
        }
    }
}

impl std::error::Error for MutantFilterError {}

pub trait MutantFilter {
    fn filter_mutants(&self, num_mutants: usize) -> Result<usize, Box<dyn error::Error>>;
}

pub struct RandomDownSampleFilter {
    /// Root of all mutants (typically `out/`). This should point to a directory
    /// containing `mutants/` and `mutants.log`
    all: String,

    /// Root of filtered mutants output. This will have the same directory
    /// structure as `all` (i.e.., filtering all will output a new `mutants.log`
    /// and a new `mutants/` directory here).
    filtered: String,
}

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

/// Ensure that the output directory structure is correct. This involves:
///
/// 1. Ensuring `root` exists and is a dir
/// 2. Ensuring `root/mutants.log` exists and is a file
///    TODO: this temporarily doesn't exist, but will be added soon!
/// 3. Ensure `root/mutants/` exists and is a dir
/// 4. Ensure that each item in `root/mutants/` is a directory with a numeric
///    name (e.g., 1/, 2/, etc)
///
/// # Arguments
///
/// * `root` - the root of the output directory structure to check

fn get_output_directory_structure(
    root: &Path,
) -> Result<OutputDirectoryStructure, Box<dyn error::Error>> {
    let mutants_log_path = root.join("mutants.log");
    let mutants_dir_path = root.join("mutants");
    if !root.is_dir() {
        return Err(Box::new(MutantFilterError::NoSuchDirectory(
            root.to_str().unwrap().into(),
        )));
    }
    if !mutants_dir_path.is_dir() {
        return Err(Box::new(MutantFilterError::NoSuchDirectory(
            mutants_dir_path.to_str().unwrap().into(),
        )));
    }
    if !mutants_log_path.is_file() {
        return Err(Box::new(MutantFilterError::NoSuchFile(
            mutants_log_path.to_str().unwrap().into(),
        )));
    }

    let mutant_paths = fs::read_dir(mutants_dir_path.clone())?;
    let mut mutants = vec![];
    for mdir in mutant_paths {
        let mdir = mdir?.path();
        if !mdir.is_dir() {
            return Err(Box::new(MutantFilterError::NoSuchDirectory(
                mdir.to_str().unwrap().into(),
            )));
        }
        mutants.push(mdir.to_path_buf());
    }
    Ok(OutputDirectoryStructure {
        root: root.to_owned(),
        mutants_log: mutants_log_path.to_owned(),
        mutants_dir: mutants_dir_path.to_owned(),
        mutants: mutants,
    })
}

impl MutantFilter for RandomDownSampleFilter {
    fn filter_mutants(&self, num_mutants: usize) -> Result<usize, Box<dyn error::Error>> {
        let root = Path::new(&self.all);
        let out = get_output_directory_structure(root)?;

        // Second: initialize the filtered mutants root. If it exists, remove it.
        // Then create a fresh filtered mutants root.

        let filtered = Path::new(&self.filtered);
        if filtered.is_file() {
            fs::remove_file(filtered)?;
        } else if filtered.is_dir() {
            fs::remove_dir_all(filtered)?;
        }

        // Third: read in all mutant directory names and randomly sample
        // `num_mutants`, without replacement. Recursively copy these to the
        // filtered mutants root

        let filtered_mutants_dir = filtered.join("mutants");
        let filtered_mutants_log = filtered.join("mutants.log");

        let mut filtered_out = OutputDirectoryStructure {
            root: filtered.to_path_buf(),
            mutants_log: filtered_mutants_log.clone(),
            mutants_dir: filtered_mutants_dir.clone(),
            mutants: vec![],
        };

        fs::create_dir(filtered)?;
        fs::create_dir(filtered_out.mutants_dir)?;

        let sampled = out
            .mutants
            .iter()
            .choose_multiple(&mut thread_rng(), num_mutants);

        let filtered_mutants = &mut filtered_out.mutants;
        for m in sampled.iter() {
            let filename = m.file_name().unwrap();
            let path = filtered_mutants_dir.join(filename);
            filtered_mutants.push(path);
        }

        // Fourth, create a new mutants log (TODO)
        Ok(filtered_mutants.len())
    }
}
