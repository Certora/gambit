/*!
* Workflow:
* Let's mainly focus on the mutation
* generation part for now.
* This tool should take as input, a solidity file,
* then compile it to generate it's AST and do various mutations of it.
* All the mutated files should be in some directory that the user will
* pass from the commandline.
!*/

use clap::Parser;
use rand::SeedableRng;
use rand_pcg::Pcg64;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs::File;

mod ast;
pub use ast::*;

mod mutation;
pub use mutation::*;

mod run;
pub use run::*;

#[derive(Debug, Clone)]
pub struct MutantGenerator {
    pub params: MutationParams,
    // will need this for randomization
    pub rng: Pcg64,
}

impl MutantGenerator {
    pub fn new(params: MutationParams) -> Self {
        MutantGenerator {
            rng: rand_pcg::Pcg64::seed_from_u64(params.seed),
            params,
        }
    }

    pub fn parse_json(&self, sol: File) -> SolAST {
        let ast_json: Value = serde_json::from_reader(sol).expect("AST json is not well-formed.");
        SolAST {
            element: Some(ast_json),
        }
    }

    pub fn run(self) {
        // TODO: this is where we will likely start adding code to actually do the mutation generation.
        // TODO: figure out how to compile, assuming json is available rn.
        for f in &self.params.filenames {
            let ast = self.parse_json(File::open(f).ok().unwrap());
            println!("{:?}", ast.get_node("exportedSymbols").get_object() == None);
        }
    }
}

/// Command line arguments for running Gambit
#[derive(Debug, Clone, Parser, Deserialize, Serialize)]
#[clap(rename_all = "kebab-case")]
pub struct MutationParams {
    /// Directory to store all mutants
    #[clap(long, default_value = "out")]
    pub outdir: String,
    /// Json file(s) to mutate
    // TODO: this should eventually be a sol file when we figure out how to compile and get the json AST properly.
    #[clap(short, long, required = true, multiple = true)]
    pub filenames: Vec<String>,
    /// Seed for random number generator
    #[clap(long, default_value = "0")]
    pub seed: u64,
    /// Num mutants
    #[clap(long, default_value = "5")]
    pub num_mutants: usize,
    /// Mutation types to enable
    #[clap(long, required = true, multiple = true)]
    pub mutations: Vec<String>,
}

/// Different commands we will support
#[derive(Parser)]
#[clap(rename_all = "kebab-case")]
pub enum Command {
    Mutate(MutationParams), // Maybe we want to do other things in the future like support checking mutants?
}

/// Entry point
fn main() {
    let _ = env_logger::builder().try_init();
    match Command::parse() {
        Command::Mutate(params) => {
            let mutant_gen = MutantGenerator::new(params);
            mutant_gen.run();
        }
    }
}
