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
use std::{
    fs::File,
    path::{Path, PathBuf},
    str::FromStr, process::ExitStatus,
};

mod ast;
pub use ast::*;
mod mutation;
pub use mutation::*;
mod run;
pub use run::*;
mod util;
pub use util::*;

#[derive(Debug, Clone)]
pub struct MutantGenerator {
    /// Params for controlling the mutants.
    pub params: MutationParams,
    /// will need this for randomization
    pub rng: Pcg64,
}

impl MutantGenerator {
    /// Initialize the MutantGenerator
    pub fn new(params: MutationParams) -> Self {
        MutantGenerator {
            rng: rand_pcg::Pcg64::seed_from_u64(params.seed),
            params,
        }
    }

    pub fn invoke_command(&self, cmd: &String, args: Vec<&str>) -> ExitStatus {
        return std::process::Command::new(&cmd)
            .args(args.iter().map(|a| a.to_string()))
            .status()
            .unwrap_or_else(|_| panic!("Failed to invoke {}.", cmd))
    }

    /// Compile the input solc files and get json ASTs.
    // TODO: need to do a more "best effort" invocation of solc.
    pub fn compile_solc(&self, sol: &String, out: PathBuf) -> SolAST {
        let norms_to_path = get_path_normals(sol);
        let norm_sol = norms_to_path.to_str().unwrap_or_else(|| {
            panic!("Could not convert the path to the sol file to a normalized version.")
        });
        let sol_path = out.join("input_json/".to_owned() + norm_sol);
        std::fs::create_dir_all(sol_path.parent().unwrap())
            .expect("Unable to create directory for storing input jsons.");
        log::info!(
            "made parent directories for writing the json file at {}.",
            sol_path.to_str().unwrap()
        );
        let _status = self.invoke_command(
            &self.params.solc,
            vec![
                "--ast-compact-json",
                sol,
                "-o",
                sol_path.to_str().unwrap(),
                "--overwrite",
            ],
        );
        let ast_fnm = Path::new(sol)
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned()
            + "_json.ast";
        let ast_path = sol_path.join(&ast_fnm);
        let json_fnm = sol_path.join(ast_fnm + ".json");
        std::fs::copy(ast_path, &json_fnm).expect("Could not copy .ast content to .json");
        let json_f = File::open(&json_fnm).unwrap_or_else(|_| {
            panic!("Cannot open the json file {}", &json_fnm.to_str().unwrap())
        });
        let ast_json: Value =
            serde_json::from_reader(json_f).expect("AST json is not well-formed.");
        SolAST {
            element: Some(ast_json),
        }
    }

    /// Generate mutations for a single file.
    fn run_one(&self, file_to_mutate: &String) {
        let rand = self.rng.clone();
        let outdir = Path::new(&self.params.outdir);
        let ast = self.compile_solc(file_to_mutate, outdir.to_path_buf());
        let mut_types = self
            .params
            .mutations
            .iter()
            .map(|m| MutationType::from_str(m).unwrap())
            .collect();

        let run_mutation = RunMutations::new(
            file_to_mutate.to_string(),
            ast,
            self.params.num_mutants,
            rand,
            outdir.to_path_buf(),
            mut_types,
        );
        log::info!("running mutations on file: {}", file_to_mutate);
        run_mutation.get_mutations();
    }

    /// Calls run_one for each file to mutate.
    pub fn run(self) {
        log::info!("starting run()");
        for f in &self.params.filenames {
            self.run_one(f);
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
    /// Solidity file(s) to mutate
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
    #[clap(long, default_value = "solc")]
    pub solc: String,
}

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
