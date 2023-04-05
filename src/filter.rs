use rand::prelude::*;
use rand_chacha::ChaCha8Rng;

/// This module downsamples mutants.
use std::error;

use crate::{Mutant, Mutator};

pub trait MutantFilter {
    fn filter_mutants(
        &self,
        mutator: &Mutator,
        num_mutants: usize,
    ) -> Result<Vec<Mutant>, Box<dyn error::Error>>;
}

pub struct RandomDownSampleFilter {
    pub(crate) seed: Option<u64>,

    /// Should filtered mutants be validated with an external compiler run? This
    /// is more expensive but disabling this option may produce invalid mutants.
    pub(crate) validate: bool,
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
        let mut mutants: Vec<(usize, Mutant)> =
            mutants.iter().map(|m| m.clone()).enumerate().collect();

        // The sampled mutants. We want to sort by the original index into
        let mut sampled: Vec<(usize, Mutant)> = vec![];

        let mut r = match self.seed {
            None => ChaCha8Rng::from_entropy(),
            Some(seed) => ChaCha8Rng::seed_from_u64(seed),
        };

        while mutants.len() > 0 && sampled.len() < num_mutants {
            // Get a random index into the current list of remaning mutants
            let idx = r.gen_range(0..mutants.len());
            let mutant = mutants.remove(idx);
            if self.validate {
                match mutator.validate_mutant(&mutant.1) {
                    Ok(true) => sampled.push(mutant),
                    _ => (),
                }
            } else {
                sampled.push(mutant);
            }
        }

        sampled.sort_by(|m1, m2| m1.0.partial_cmp(&m2.0).unwrap());

        Ok(sampled.iter().map(|m| m.1.clone()).collect())
    }
}
