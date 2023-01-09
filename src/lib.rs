use clap::{Parser, ValueEnum};
use rand::SeedableRng;
use rand_pcg::Pcg64;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use core::panic;
use std::collections::HashSet;
use std::fmt::Debug;
use std::io::BufReader;
use std::{fs, io};
use std::{
    fs::File,
    path::{Path, PathBuf},
};

mod ast;
pub use ast::*;
mod mutation;
pub use mutation::*;
mod run;
pub use run::*;
mod util;
pub use util::*;

/// temporary paths for compiling mutants.
static TMP: &str = "tmp.sol";

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

    /// A helper function to create the directory where the
    /// AST (.ast) and it's json representation (.ast.json)
    /// are stored.
    /// This returns the directory, and both the path to the .ast and the .ast.json.
    fn mk_ast_dir(&self, sol: &String, out: PathBuf) -> (PathBuf, PathBuf, PathBuf) {
        let norms_of_path =
            get_path_normals(sol).unwrap_or_else(|| panic!("Path to sol file is broken"));
        let extension = norms_of_path.extension();
        if extension.is_none() || !extension.unwrap().eq("sol") {
            panic!("{} is not a solidity source file.", sol);
        }
        let sol_ast_dir = out.join(
            "input_json/".to_owned()
                + norms_of_path
                    .to_str()
                    .unwrap_or_else(|| panic!("Path is not valid.")),
        );
        let ast_fnm = Path::new(sol)
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned()
            + "_json.ast";
        let ast_path = sol_ast_dir.join(&ast_fnm);
        let json_path = sol_ast_dir.join(ast_fnm + ".json");
        (sol_ast_dir, ast_path, json_path)
    }

    /// This method compiles an input solc file to get the json AST.
    /// For simple examples, it simply sufficies to run the right version of
    /// solc on the file but for more complex examples,
    /// it uses the `--solc-basepath` flag (that the user must provide in the config file)
    /// to set the `--base-path` when invoking the Solidity compiler.
    /// You can read more about it in the [Solidity documentation](https://docs.soliditylang.org/en/v0.8.17/path-resolution.html#base-path-and-include-paths).
    pub fn compile_solc(
        &self,
        sol: &String,
        out: PathBuf,
    ) -> Result<SolAST, Box<dyn std::error::Error>> {
        let (sol_ast_dir, ast_path, json_path) = self.mk_ast_dir(sol, out);
        if !ast_path.exists() || !json_path.exists() {
            std::fs::create_dir_all(sol_ast_dir.parent().unwrap())?;
            log::info!(
                "made parent directories for writing the json ast at {}.",
                sol_ast_dir.to_str().unwrap()
            );
            let mut flags: Vec<&str> = vec![
                "--ast-compact-json",
                sol,
                "-o",
                sol_ast_dir.to_str().unwrap(),
                "--overwrite",
            ];

            if self.params.solc_basepath.is_some() {
                flags.push("--base-path");
                flags.push(self.params.solc_basepath.as_ref().unwrap());
            }

            if invoke_command(&self.params.solc, flags)?
                .0
                .unwrap_or_else(|| panic!("solc terminated with a signal."))
                != 0
            {
                panic!("Failed to compile source. Maybe try with a different version of solc (e.g., --solc solc8.10)")
            }

            std::fs::copy(ast_path, &json_path)?;
        }
        let json_f = File::open(&json_path)?;
        let ast_json: Value = serde_json::from_reader(json_f)?;
        Ok(SolAST {
            element: Some(ast_json),
            contract: None,
        })
    }

    /// Create a directory for saving the mutants for a given
    /// file `fnm`. All mutant files will be dumped here.
    fn mk_mutant_dir(&self, fnm: &str) -> io::Result<()> {
        let norm_path = get_path_normals(fnm);
        assert!(norm_path.is_some());
        let mut_dir = PathBuf::from(&self.params.outdir).join(norm_path.unwrap());
        if let Some(pd) = mut_dir.parent() {
            if pd.is_dir() {
                fs::remove_dir_all(pd)?;
            }
        }
        std::fs::create_dir_all(mut_dir.parent().unwrap())?;
        Ok(())
    }

    /// Create directories for mutants from a json config file.
    /// This is used when Gambit is run using a config file as opposed
    /// to individual solidity files using the `-f` flag.
    fn mutant_dirs_from_json(&self) -> io::Result<()> {
        let f = File::open(&self.params.json.as_ref().unwrap())?;
        let config: Value = serde_json::from_reader(BufReader::new(f))?;
        match config {
            Value::Array(elems) => {
                let mut paths = HashSet::new();
                for e in elems {
                    paths.insert(e["filename"].as_str().unwrap().to_string());
                }
                paths.iter().for_each(|p| {
                    self.mk_mutant_dir(p).ok();
                });
            }
            Value::Object(o) => {
                self.mk_mutant_dir(o["filename"].as_str().unwrap())?;
            }
            _ => panic!("Ill-formed json."),
        }
        Ok(())
    }

    /// Generate mutations for a single file.
    /// Irrespective of how Gambit is used,
    /// this is the method which performs mutations
    /// on a single solidity file.
    fn run_one(
        &self,
        file_to_mutate: &String,
        muts: Option<Vec<String>>,
        funcs: Option<Vec<String>>,
        contract: Option<String>,
    ) -> io::Result<()> {
        let rand = self.rng.clone();
        let outdir = Path::new(&self.params.outdir);
        let ast = self
            .compile_solc(file_to_mutate, outdir.to_path_buf())
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
            fnm: file_to_mutate.into(),
            node: ast,
            num_mutants: self.params.num_mutants,
            rand,
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
            if self.params.solc_basepath.is_some() {
                let f_path = PathBuf::from(file_to_mutate.as_str());
                let parent_of_fnm = f_path.parent().unwrap_or_else(|| {
                    panic!("Parent being None here means no file is being mutated.")
                });
                let tmp = parent_of_fnm.join(TMP);
                std::fs::write(&tmp, mutant)?;
                flags.push(tmp.to_str().as_ref().unwrap());
                flags.push("--base-path");
                flags.push(self.params.solc_basepath.as_ref().unwrap());
                (valid, _, _) = invoke_command(&self.params.solc, flags)?;
                if tmp.exists() {
                    let _ = std::fs::remove_file(tmp);
                }
            } else {
                std::fs::write(&TMP, mutant)?;
                flags.push(TMP);
                (valid, _, _) = invoke_command(&self.params.solc, flags)?;
                std::fs::remove_file(TMP)?;
            }
            match valid {
                Some(n) => Ok(n == 0),
                None => Ok(false),
            }
        };
        match run_mutation.get_mutations(is_valid) {
            Ok(_) => (),
            Err(_) => panic!("Mutation generation failed."),
        };
        Ok(())
    }

    /// Run Gambit from a json config file.
    /// You can find examples of comfig files under `benchmarks/config-jsons/`.
    /// A configuration allows the user to have more control on
    /// which contracts and functions to mutate and using which kinds of mutations.
    fn run_from_config(&mut self, cfg: &String) -> io::Result<()> {
        let cfg = Path::new(cfg);
        if !cfg.is_file() || !cfg.extension().unwrap().eq("json") {
           panic!("Must pass a .json config file with the --json argument or gambit-cfg alias. You can use the gambit alias instead!"); 
        }
        self.mutant_dirs_from_json()?;
        let f = File::open(cfg)?;
        let config: Value = serde_json::from_reader(BufReader::new(f))?;
        let mut process_single_file = |v: &Value| -> io::Result<()> {
            if let Some(filename) = &v.get("filename") {
                let mut funcs_to_mutate: Option<Vec<String>> = None;
                let mut selected_muts: Option<Vec<String>> = None;
                let fnm = filename.as_str().unwrap();
                if let Some(num) = &v.get("num-mutants") {
                    self.params.num_mutants = num.as_i64().unwrap();
                }
                if let Some(solc) = &v.get("solc") {
                    self.params.solc = solc.as_str().unwrap().to_string();
                }
                if let Some(solc_basepath) = &v.get("solc-basepath") {
                    self.params.solc_basepath = solc_basepath.as_str().unwrap().to_string().into();
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
                self.run_one(&fnm.to_string(), selected_muts, funcs_to_mutate, contract)?;
            }
            Ok(())
        };
        match config {
            Value::Array(elems) => {
                for elem in elems {
                    process_single_file(&elem)?;
                }
            }
            Value::Object(_) => {
                process_single_file(&config)?;
            }
            _ => panic!("Ill-formed json."),
        }
        Ok(())
    }

    /// Main runner that either runs Gambit on one or more .sol
    /// files passed from the command line,
    /// or using a config file (see examples under `benchmarks/config-jsons/`).
    pub fn run(&mut self) -> io::Result<()> {
        log::info!("starting run()");
        let files = &self.params.filename;
        let json = &self.params.json.clone();
        if files.is_some() {
            for f in files.as_ref().unwrap() {
                self.mk_mutant_dir(&f.to_string())?;
                self.run_one(f, None, None, None)?;
            }
        } else if json.is_some() {
            self.run_from_config(json.as_ref().unwrap())?;
        } else {
            panic!("Must provide either --filename file.sol or --json config.json.")
        }
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
pub struct MutationParams {
    /// Json file with config
    #[arg(long, short, conflicts_with = "filename")]
    pub json: Option<String>,
    /// File to mutate
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
}

#[derive(Parser)]
#[clap(rename_all = "kebab-case")]
pub enum Command {
    Mutate(MutationParams), // Maybe we want to do other things in the future like support checking mutants?
}
