use rand::prelude::*;
use rand_chacha::ChaCha8Rng;
use std::error;

use crate::{Mutant, Mutator};

/// This module downsamples mutants.

/// Implement this trait to filter mutants after they have been created.
pub trait MutantFilter {
    /// Filter the mutants of a mutator, validating them via compilation if
    /// `self.validate()` returns `true`.
    fn filter_mutants(
        &self,
        mutator: &Mutator,
        num_mutants: usize,
    ) -> Result<Vec<Mutant>, Box<dyn error::Error>>;

    fn validate(&self) -> bool;
}

/// This struct randomly downsamples mutants.
pub struct RandomDownSampleFilter {
    pub(crate) seed: Option<u64>,

    /// Should filtered mutants be validated with an external compiler run? This
    /// is more expensive but disabling this option may produce invalid mutants.
    validate: bool,
}

impl RandomDownSampleFilter {
    pub fn new(seed: Option<u64>, validate: bool) -> Self {
        Self { seed, validate }
    }
}

impl MutantFilter for RandomDownSampleFilter {
    fn filter_mutants(
        &self,
        mutator: &Mutator,
        num_mutants: usize,
    ) -> Result<Vec<Mutant>, Box<dyn error::Error>> {
        // Make a copy that we can mutate
        let mutants = mutator.mutants();
        let mut mutants: Vec<(usize, Mutant)> = mutants.iter().cloned().enumerate().collect();

        // The sampled mutants. We want to sort by the original index into
        let mut sampled: Vec<(usize, Mutant)> = vec![];

        let mut r = match self.seed {
            None => ChaCha8Rng::from_entropy(),
            Some(seed) => ChaCha8Rng::seed_from_u64(seed),
        };

        while !mutants.is_empty() && sampled.len() < num_mutants {
            // Get a random index into the current list of remaning mutants
            let idx = r.gen_range(0..mutants.len());
            let mutant = mutants.remove(idx);
            if self.validate() {
                if let Ok(true) = mutator.validate_mutant(&mutant.1) {
                    sampled.push(mutant)
                }
            } else {
                sampled.push(mutant);
            }
        }

        sampled.sort_by(|m1, m2| m1.0.partial_cmp(&m2.0).unwrap());

        Ok(sampled.iter().map(|m| m.1.clone()).collect())
    }

    fn validate(&self) -> bool {
        self.validate
    }
}
