use clap::Parser;
use serde::{Deserialize, Serialize};

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

    /// If specified, randomly downsamples the number of mutants
    #[arg(long, short, default_value = None)]
    pub num_mutants: Option<usize>,

    /// Specify a random seed for down sampling
    #[arg(long, short, default_value = None)]
    pub seed: Option<u64>,

    /// Output directory to place results of mutation
    #[arg(long, short, default_value = "out")]
    pub outdir: String,

    /// Log mutants
    #[arg(long, default_value = "true")]
    pub log_mutants: bool,

    /// Export full mutant sources
    #[arg(long, default_value = "false")]
    pub export_mutants: bool,

    /// Solidity binary name, e.g., --solc solc8.10, --solc 7.5, etc.
    #[arg(long, default_value = "solc")]
    pub solc: String,

    /// Specify function names to mutate
    #[arg(long)]
    pub fns_to_mutate: Option<Vec<String>>,

    /// Specify a contract to mutate
    #[arg(long)]
    pub contract_to_mutate: Option<String>,

    /// Basepath argument to solc
    #[arg(long)]
    pub solc_basepath: Option<String>,

    /// Allowpath argument to solc
    #[arg(long)]
    pub solc_allowpaths: Option<Vec<String>>,

    /// Solidity remappings
    #[arg(long)]
    pub solc_remapping: Option<Vec<String>>,

    /// Specify this
    #[arg(long, default_value = "true")]
    pub validate: bool,
}

#[derive(Parser)]
#[clap(rename_all = "kebab-case")]
pub enum Command {
    Mutate(MutateParams), // Maybe we want to do other things in the future like support checking mutants?
}
