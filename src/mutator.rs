/// This module is responsible for high level logic of running mutation over
/// Solidity programs.
use crate::{read_source, source::Source, MutationType};
use std::{
    error,
    path::{Path, PathBuf},
    rc::Rc,
};

/// The configuration for a mutator, this details how a given mutation run
/// should operate
#[derive(Debug, Clone)]
pub struct MutatorConfig {}

/// This struct describes a mutant.
#[derive(Debug, Clone)]
pub struct Mutant {
    /// The original program's source
    pub source: Rc<Source>,

    /// The mutation operator that was applied to generate this mutant
    pub op: MutationType,

    /// The index into the program source marking the beginning (inclusive) of
    /// the source to be replaced
    pub start: usize,

    /// The index into the program source marking the end (inclusive) of the
    /// source to be replaced
    pub end: usize,

    /// The string replacement
    pub repl: String,
}

/// The mutator performs the actual logic of mutating a program, writes
#[derive(Debug)]
pub struct Mutator {
    /// Configuration for this mutator
    pub conf: MutatorConfig,

    /// The original sources
    pub sources: Vec<Rc<Source>>,

    /// The mutants, in order of generation
    pub mutants: Vec<Mutant>,

    /// A temporary directory to store intermediate work
    tmp: PathBuf,
}

impl Mutator {
    /// Run all mutations! This is the main external entry point into mutation. This
    /// 1. Mutates each file
    /// 2. Optionally downsamples (default: no) the mutants if a Filter is provided
    /// 3. Optionally validates (default: yes) all generated/filtered mutants
    /// 4. Writes mutants to disk
    ///    1. Writes the mutant log
    ///    2. Writes each mutant's diff
    ///    3. Optionally writes each mutant's source (default: false)
    pub fn mutate(&mut self) -> Result<(), Box<dyn error::Error>> {
        let mut mutants: Vec<Mutant> = vec![];

        for source in self.sources.iter() {
            let mut file_mutants = self.mutate_file(source)?;
            mutants.append(&mut file_mutants);
        }

        self.mutants.append(&mut mutants);
        panic!()
    }

    /// Mutate a single file.
    fn mutate_file(&self, source: &Source) -> Result<Vec<Mutant>, Box<dyn error::Error>> {
        let filename = source.filename();
        let contents = source.contents()?;
        todo!()
    }
}
