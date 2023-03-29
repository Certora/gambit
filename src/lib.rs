use clap::{Parser, ValueEnum};
use core::panic;
use global_counter::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_json::Value::Array;
use std::fmt::Debug;
use std::io;
use std::io::BufReader;
use std::io::Write;
use std::{
    fs::File,
    path::{Path, PathBuf},
};

mod ast;
pub use ast::*;
mod compile;
pub use compile::*;
mod mutation;
pub use mutation::*;
mod mutator;
pub use mutator::*;
mod filter;
pub use filter::*;
mod run;
pub use run::*;
mod source;
pub use source::*;
mod util;
pub use util::*;

/// temporary paths for compiling mutants.
static TMP: &str = "tmp.sol";
static INPUT_JSON: &str = "input_json";
static BASEPATH: &str = "--base-path";
static ALLOWPATH: &str = "--allow-paths";
static DOT_JSON: &str = ".json";
static FILENAME: &str = "filename";

// TODO: This should belong to MutantGenerator
global_counter!(MUTANT_COUNTER, u64, 0);

/// Produce the next available mutant id and increment the counter
pub fn next_mid() -> u64 {
    MUTANT_COUNTER.inc();
    let id = MUTANT_COUNTER.get_cloned();
    id
}

pub fn current_mid() -> u64 {
    MUTANT_COUNTER.get_cloned()
}

#[derive(Debug, Clone)]
pub struct MutantGenerator {
    /// Params for controlling the mutants.
    pub params: MutateParams,
}

impl MutantGenerator {
    /// Initialize the MutantGenerator
    pub fn new(params: MutateParams) -> Self {
        MutantGenerator { params }
    }

    /// A helper function to create the directory where the AST (.ast) and it's
    /// json representation (.ast.json) are stored.
    ///
    /// # Arguments
    ///
    /// * `sol` - Solidity file that is going to be compiled
    /// * `out` - The output directory
    ///
    /// # Returns
    ///
    /// This returns a 3-tuple:
    /// * `sol_ast_dir` - the path to the directory of the solidity AST
    /// * `ast_path` - the solidity AST file (contained inside `sol_ast_dir`)
    /// * `json_path` - the solidity AST JSON file (contained inside
    ///   `sol_ast_dir`)
    fn mk_ast_dir(&self, sol: &String, out: PathBuf) -> io::Result<(PathBuf, PathBuf, PathBuf)> {
        let sol_path = Path::new(sol);
        let extension = sol_path.extension();
        if extension.is_none() || !extension.unwrap().eq("sol") {
            panic!("{} is not a solidity source file.", sol);
        }
        let sol_ast_dir = out.join(INPUT_JSON.to_owned()).join(sol);
        let ast_fnm = Path::new(sol)
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned()
            + "_json.ast";
        let ast_path = sol_ast_dir.join(&ast_fnm);
        let json_path = sol_ast_dir.join(ast_fnm + DOT_JSON);
        Ok((sol_ast_dir, ast_path, json_path))
    }

    /// This method compiles an input sol file to get the json AST.
    /// For simple examples, it simply sufficies to run the right version of
    /// solc on the file but for more complex examples,
    /// it uses the `--solc-basepath`,
    ///  `--solc-allowpaths` and `remappings` flags (that the user must provide in the config file)
    /// to set the `--base-path`, `--allow-paths`, and node-modules when invoking the Solidity compiler.
    /// You can read more about it in the [Solidity documentation](https://docs.soliditylang.org/en/v0.8.17/path-resolution.html#base-path-and-include-paths).
    pub fn compile_sol(
        &self,
        sol: &String,
        out: PathBuf,
    ) -> Result<SolAST, Box<dyn std::error::Error>> {
        let (sol_ast_dir, ast_path, json_path) = self.mk_ast_dir(sol, out)?;
        // if !ast_path.exists() || !json_path.exists() {
        std::fs::create_dir_all(sol_ast_dir.parent().unwrap())?;
        log::info!(
            "made parent directories for writing the json ast at {}.",
            sol_ast_dir.to_str().unwrap()
        );
        let mut flags: Vec<&str> = vec![
            "--ast-compact-json",
            sol,
            "-o",
            // "--optimize",
            sol_ast_dir.to_str().unwrap(),
            "--overwrite",
        ];

        if self.params.solc_basepath.is_some() {
            flags.push(BASEPATH);
            flags.push(self.params.solc_basepath.as_ref().unwrap());
        }

        if let Some(remaps) = &self.params.solc_allowpaths {
            flags.push(ALLOWPATH);
            for r in remaps {
                flags.push(r);
            }
        }

        if let Some(remaps) = &self.params.solc_remapping {
            for r in remaps {
                flags.push(r);
            }
        }
        let pretty_flags = flags
            .iter()
            .map(|x| x.to_string())
            .collect::<Vec<_>>()
            .join(" ");

        if invoke_command(&self.params.solc, flags)?
            .0
            .unwrap_or_else(|| panic!("solc terminated with a signal."))
            != 0
        {
            eprintln!("Solidity compiler failed unexpectedly. For more details, try running the following command from your terminal:");
            eprintln!("`{} {}`", &self.params.solc, pretty_flags);
            std::process::exit(1)
        }

        log::info!(
            "Successfully wrote ASTs to {}",
            sol_ast_dir.to_str().unwrap()
        );
        std::fs::copy(ast_path, &json_path)?;
        // } else {
        //     log::info!(
        //         ".ast and .ast.json both exist at {:?} and {:?}.",
        //         ast_path,
        //         json_path
        //     );
        // }
        let json_f = File::open(&json_path)?;
        let ast_json: Value = serde_json::from_reader(json_f)?;
        Ok(SolAST {
            element: Some(ast_json),
            contract: None,
        })
    }

    /// Generate mutations for a single file.
    ///
    /// Irrespective of how Gambit is used, this is the method which performs
    /// mutations on a single solidity file.
    ///
    /// # Arguments
    ///
    /// * `file_to_mutate` - Path to the file we are mutating
    /// * `muts` - An optional list of mutation operators to be applied: if
    ///   `None`, use the default set of operators
    /// * `funcs` - An optional list of functions to mutate: if `None`, all
    ///   functions will be mutated
    /// * `contract` - TODO: what is this?
    fn mutate_file(
        &self,
        file_to_mutate: &String,
        muts: Option<Vec<String>>,
        funcs: Option<Vec<String>>,
        contract: Option<String>,
    ) -> io::Result<Vec<serde_json::Value>> {
        let outdir = Path::new(&self.params.outdir);
        let ast = self
            .compile_sol(file_to_mutate, outdir.to_path_buf())
            .ok()
            .unwrap();
        let mut_types = muts.map_or(MutationType::value_variants().to_vec(), |ms| {
            ms.iter()
                .map(|m| {
                    MutationType::from_str(m, true)
                        .unwrap_or_else(|_| panic!("Could not generate mutant type from {}.", m))
                })
                .collect()
        });

        let run_mutation = RunMutations {
            filename: file_to_mutate.into(),
            mutation_root: ast,
            out: outdir.to_path_buf(),
            mutation_types: mut_types,
            funcs_to_mutate: funcs,
            contract,
        };
        log::info!("running mutations on file: {}", file_to_mutate);

        // This closure checks whether a mutant is valid
        // by invoking the solidity compiler on it.
        let is_valid = |mutant: &str| -> Result<bool, Box<dyn std::error::Error>> {
            let mut flags: Vec<&str> = vec![];
            let valid;
            let f_path = PathBuf::from(file_to_mutate.as_str());
            let parent_of_fnm = f_path.parent().unwrap_or_else(|| {
                panic!("Parent being None here means no file is being mutated.")
            });
            let tmp = parent_of_fnm.join(TMP);
            std::fs::write(&tmp, mutant)?;
            flags.push(tmp.to_str().as_ref().unwrap());
            if let Some(bp) = &self.params.solc_basepath {
                flags.push(BASEPATH);
                flags.push(bp);
            }

            if let Some(aps) = &self.params.solc_allowpaths {
                flags.push(ALLOWPATH);
                for a in aps {
                    flags.push(a);
                }
            }

            if let Some(remaps) = &self.params.solc_remapping {
                for r in remaps {
                    flags.push(r);
                }
            }
            (valid, _, _) = invoke_command(&self.params.solc, flags)?;
            if tmp.exists() {
                let _ = std::fs::remove_file(tmp);
            }
            match valid {
                Some(n) => Ok(n == 0),
                None => Ok(false),
            }
        };
        match run_mutation.get_mutations(is_valid) {
            Ok(map) => Ok(map.iter().map(|(_, y)| y.clone()).collect()),
            Err(_) => panic!("Mutation generation failed."),
        }
    }

    fn process_single_file(&mut self, v: &Value, cfg: &str) -> io::Result<Vec<serde_json::Value>> {
        if let Some(filename) = &v.get(FILENAME) {
            let mut funcs_to_mutate: Option<Vec<String>> = None;
            let mut selected_muts: Option<Vec<String>> = None;
            let fnm = resolve_path_from_str(cfg, filename.as_str().unwrap()); // ok to unwrap because we have prior checks.
            if let Some(num) = &v.get("num-mutants") {
                self.params.num_mutants = num.as_i64().unwrap();
            }
            if let Some(solc) = &v.get("solc") {
                self.params.solc = solc.as_str().unwrap().to_string();
            }
            if let Some(solc_basepath) = &v.get("solc-basepath") {
                self.params.solc_basepath =
                    resolve_path_from_str(cfg, solc_basepath.as_str().unwrap()).into();
            }
            if let Some(allowed_paths) = &v.get("solc-allowpaths") {
                let allowed: Vec<String> = allowed_paths
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| resolve_path_from_str(cfg, v.as_str().unwrap()))
                    .collect();
                if !allowed.is_empty() {
                    self.params.solc_allowpaths = allowed.into();
                }
            }
            if let Some(remap_args) = &v.get("remappings") {
                let remaps: Vec<String> = remap_args
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| repair_remapping(v.as_str().unwrap(), cfg))
                    .collect();
                if !remaps.is_empty() {
                    self.params.solc_remapping = remaps.into();
                }
            }
            if let Some(seed) = &v.get("seed") {
                self.params.seed = seed.as_u64().unwrap();
            }
            let contract: Option<String> =
                v.get("contract").map(|v| v.as_str().unwrap().to_string());

            if let Some(muts) = &v.get("mutations") {
                let mutts: Vec<String> = muts
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_str().unwrap().to_string())
                    .collect();
                if !mutts.is_empty() {
                    selected_muts = mutts.into();
                }
            }
            if let Some(funcs) = &v.get("functions") {
                let fs: Vec<String> = funcs
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_str().unwrap().to_string())
                    .collect();
                if !fs.is_empty() {
                    funcs_to_mutate = fs.into();
                }
            }
            Ok(self.mutate_file(&fnm, selected_muts, funcs_to_mutate, contract)?)
        } else {
            Ok(vec![])
        }
    }

    /// Run Gambit from a json config file.
    /// You can find examples of config files under `benchmarks/config-jsons/`.
    /// A configuration allows the user to have more control on
    /// which contracts and functions to mutate and using which kinds of mutations.
    fn run_from_config(&mut self, cfg: &String) -> io::Result<Vec<serde_json::Value>> {
        let cfg_path = Path::new(cfg);
        if !cfg_path.is_file() || !cfg_path.extension().unwrap().eq("json") {
            panic!("Must pass a .json config file with the --json argument or gambit-cfg alias. You can use the gambit alias instead!");
        }
        let config: Value = serde_json::from_reader(BufReader::new(File::open(cfg_path)?))?;
        match config {
            Value::Array(elems) => {
                let mut results_json = vec![];
                for elem in elems {
                    let mut new_results = self.process_single_file(&elem, cfg)?;
                    results_json.append(&mut new_results)
                }
                Ok(results_json)
            }
            Value::Object(_) => self.process_single_file(&config, cfg),
            _ => panic!("Ill-formed json."),
        }
    }

    /// Main runner that either runs Gambit on one or more .sol
    /// files passed from the command line,
    /// or using a config file (see examples under `benchmarks/config-jsons/`).
    pub fn run(&mut self) -> io::Result<()> {
        log::info!("starting run()");
        let files = &self.params.filename;
        let json = &self.params.json.clone();
        let results = if files.is_some() {
            log::info!("running with solidity files");
            let mut results_json: Vec<serde_json::Value> = vec![];
            for f in files.as_ref().unwrap() {
                let mut new_results = self.mutate_file(f, None, None, None)?;
                results_json.append(&mut new_results)
            }
            results_json
        } else if json.is_some() {
            log::info!("running from a json config file");
            self.run_from_config(json.as_ref().unwrap())?
        } else {
            panic!("Must provide either --filename file.sol or --json config.json.")
        };
        let json_string = Array(results).to_string();
        let results_fn = self.params.outdir.to_owned() + "/gambit_result" + DOT_JSON;
        let results_path = Path::new(&results_fn);
        let mut results_file = File::create(results_path)?;
        File::write(&mut results_file, json_string.as_bytes())?;
        Ok(())
    }
}

///
/// Command line arguments for running Gambit.
/// Following are the main ways to run it.
///
///    1. cargo gambit path/to/file.sol: this will apply all mutations to file.sol.
///
///    2. cargo run --release -- mutate -f path/to/file1.sol -f path/to/file2.sol: this will apply all mutations to file1.sol and file2.sol.
///
///    3. cargo gambit-cfg path/to/config.json: this gives the user finer control on what functions in
///       which files, contracts to mutate using which types of mutations.
///
#[derive(Debug, Clone, Parser, Deserialize, Serialize)]
#[command(rename_all = "kebab-case")]
pub struct MutateParams {
    /// Json file with config
    #[arg(long, short, conflicts_with = "filename")]
    pub json: Option<String>,
    /// Files to mutate
    #[arg(long, short, conflicts_with = "json")]
    pub filename: Option<Vec<String>>,
    /// Number of mutants
    #[arg(long, short, default_value = "5")]
    pub num_mutants: i64,
    /// Directory to store all mutants
    #[arg(long, short, default_value = "out")]
    pub outdir: String,
    /// Seed for random number generator
    #[arg(long, short, default_value = "0")]
    pub seed: u64,
    /// Solidity binary name, e.g., --solc solc8.10, --solc 7.5, etc.
    #[arg(long, default_value = "solc")]
    pub solc: String,
    /// Basepath argument to solc
    #[arg(long)]
    pub solc_basepath: Option<String>,
    /// Allowpath argument to solc
    #[arg(long)]
    pub solc_allowpaths: Option<Vec<String>>,
    /// Solidity remappings
    #[arg(long)]
    pub solc_remapping: Option<Vec<String>>,
}

#[derive(Parser)]
#[clap(rename_all = "kebab-case")]
pub enum Command {
    Mutate(MutateParams), // Maybe we want to do other things in the future like support checking mutants?
}
