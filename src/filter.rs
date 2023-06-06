use rand::prelude::*;
use rand_chacha::ChaCha8Rng;
use std::error;
use tempfile::{tempdir, NamedTempFile};

use crate::{Mutant, MutantWriter, Mutator, Solc};

/// This module downsamples mutants.

/// Implement this trait to filter mutants after they have been created.
pub trait MutantFilter {
    /// Filter the mutants of a mutator, validating them via compilation if
    /// `self.validate()` returns `true`. When successful, return an
    /// Ok((valid-mutants, invalid-mutants))
    fn filter_mutants(
        &self,
        mutator: &Mutator,
        num_mutants: usize,
    ) -> Result<(Vec<Mutant>, Vec<Mutant>), Box<dyn error::Error>>;

    fn validate(&self) -> bool;
}

/// This struct randomly downsamples mutants.
pub struct RandomDownSampleFilter {
    pub(crate) seed: Option<u64>,

    /// Should filtered mutants be validated with an external compiler run? This
    /// is more expensive but disabling this option may produce invalid mutants.
    validate: bool,

    validator: Validator,
}

impl RandomDownSampleFilter {
    pub fn new(seed: Option<u64>, validate: bool, validator: Validator) -> Self {
        Self {
            seed,
            validate,
            validator,
        }
    }
}

impl MutantFilter for RandomDownSampleFilter {
    fn filter_mutants(
        &self,
        mutator: &Mutator,
        num_mutants: usize,
    ) -> Result<(Vec<Mutant>, Vec<Mutant>), Box<dyn error::Error>> {
        // Make a copy that we can mutate
        let mut mutants: Vec<(usize, Mutant)> =
            mutator.mutants().iter().cloned().enumerate().collect();

        // The sampled mutants. We want to sort by the original index into
        let mut sampled: Vec<(usize, Mutant)> = vec![];
        let mut invalid: Vec<(usize, Mutant)> = vec![];

        let mut r = match self.seed {
            None => ChaCha8Rng::from_entropy(),
            Some(seed) => ChaCha8Rng::seed_from_u64(seed),
        };

        while !mutants.is_empty() && sampled.len() < num_mutants {
            // Get a random index into the current list of remaning mutants
            let idx = r.gen_range(0..mutants.len());
            let mutant = mutants.remove(idx);
            if self.validate() {
                if let Ok(true) = self.validator.validate_mutant(&mutant.1) {
                    sampled.push(mutant)
                } else {
                    invalid.push(mutant)
                }
            } else {
                sampled.push(mutant);
            }
        }

        sampled.sort_by(|m1, m2| m1.0.partial_cmp(&m2.0).unwrap());

        Ok((
            sampled.iter().map(|m| m.1.clone()).collect(),
            invalid.iter().map(|m| m.1.clone()).collect(),
        ))
    }

    fn validate(&self) -> bool {
        self.validate
    }
}

/// Responsible for mutant validation logic
pub struct Validator {
    pub solc: Solc,
}

impl Validator {
    /// validate a mutant by writing it to disk and compiling it. If compilation
    /// fails then this is an invalid mutant.
    pub fn validate_mutant(&self, mutant: &Mutant) -> Result<bool, Box<dyn error::Error>> {
        let source_filename = mutant.source.filename();
        let source_parent_dir = source_filename.parent().unwrap();
        let mutant_file = NamedTempFile::new_in(source_parent_dir)?;
        let mutant_file_path = mutant_file.path();
        log::debug!(
            "Validating mutant of {}: copying mutated code to {}",
            source_filename.display(),
            mutant_file_path.display()
        );
        let dir = tempdir()?;
        MutantWriter::write_mutant_to_file(mutant_file_path, mutant)?;
        let was_success = match self.solc.compile(mutant_file_path, dir.path()) {
            Ok((code, _, _)) => code == 0,
            Err(_) => false,
        };
        Ok(was_success)
    }

    /// Return a tuple of (valid-mutants, invalid-mutants)
    pub fn get_valid_mutants(&self, mutants: &[Mutant]) -> (Vec<Mutant>, Vec<Mutant>) {
        log::info!("Validating mutants...");
        let mut valid_mutants = vec![];
        let mut invalid_mutants: Vec<Mutant> = vec![];
        for m in mutants.iter() {
            if let Ok(true) = self.validate_mutant(m) {
                valid_mutants.push(m.clone())
            } else {
                invalid_mutants.push(m.clone())
            }
        }
        (valid_mutants, invalid_mutants)
    }
}
