use clap::Parser;
use serde::{Deserialize, Serialize};

static DEFAULT_NO_EXPORT_MUTANTS: bool = false;
static DEFAULT_NO_OVERWRITE: bool = false;
static DEFAULT_RANDOM_SEED: bool = false;
static DEFAULT_SEED: u64 = 0;
static DEFAULT_SKIP_VALIDATE: bool = false;
static DEFAULT_SOLC: &str = "solc";

fn default_no_export_mutants() -> bool {
    DEFAULT_NO_EXPORT_MUTANTS
}

fn default_no_overwrite() -> bool {
    DEFAULT_NO_OVERWRITE
}

fn default_random_seed() -> bool {
    DEFAULT_RANDOM_SEED
}

fn default_seed() -> u64 {
    DEFAULT_SEED
}

fn default_skip_validate() -> bool {
    DEFAULT_SKIP_VALIDATE
}

fn default_solc() -> String {
    DEFAULT_SOLC.to_string()
}

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
    pub filename: Option<String>,

    /// If specified, randomly downsamples the number of mutants
    #[arg(long, short, default_value = None)]
    pub num_mutants: Option<usize>,

    /// Use a random seed instead of the specified seed. This will override any
    /// value passed in with the `--seed` flag
    #[arg(long, default_value = "false")]
    #[serde(default = "default_random_seed")]
    pub random_seed: bool,

    /// Specify a seed for randomized down sampling. By default seed=0 is used
    /// and is deterministic, but nondeterminism can be enabled with the
    /// `--random-seed` flag
    #[arg(long, short, default_value = "0")]
    #[serde(default = "default_seed")]
    pub seed: u64,

    /// Output directory to place results of mutation
    #[arg(long, short, default_value = crate::DEFAULT_GAMBIT_OUTPUT_DIRECTORY)]
    #[serde(default = "crate::default_gambit_output_directory")]
    pub outdir: String,

    /// Specify the mutation operators
    #[arg(long, num_args(1..))]
    pub mutations: Option<Vec<String>>,

    /// Skip mutant export
    #[arg(long, default_value_t = DEFAULT_NO_EXPORT_MUTANTS)]
    #[serde(default = "default_no_export_mutants")]
    pub no_export: bool,

    /// Overwrite output directory (by default, a warning will print and this will exit)
    #[arg(long, default_value = "false")]
    #[serde(default = "default_no_overwrite")]
    pub no_overwrite: bool,

    /// Solidity binary name, e.g., --solc solc8.10, --solc 7.5, etc.
    #[arg(long, default_value = "solc")]
    #[serde(default = "default_solc")]
    pub solc: String,

    /// Specify function names to mutate
    #[arg(long)]
    pub functions: Option<Vec<String>>,

    /// Specify a contract to mutate
    #[arg(long)]
    pub contract: Option<String>,

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
    #[arg(long, default_value = "false")]
    #[serde(default = "default_skip_validate")]
    pub skip_validate: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GambitConfigFile {
    pub configurations: Vec<MutateParams>,
}

#[derive(Parser)]
#[clap(rename_all = "kebab-case")]
pub enum Command {
    Mutate(MutateParams), // Maybe we want to do other things in the future like support checking mutants?
    Summary(SummaryParams),
}

/// Summarize mutants

#[derive(Debug, Clone, Parser, Deserialize, Serialize)]
#[command(rename_all = "kebab-case")]
pub struct SummaryParams {
    /// Print summaries of the specified mutant IDs (these IDs correspond to the
    /// "id" field in `gambit_results.json`). Multiple MIDs can be specified.
    /// If `--all` is specified, this is ignored.
    #[arg(long, default_value = None, num_args(0..), conflicts_with = "all")]
    pub mids: Option<Vec<String>>,

    /// Report all mutants
    #[arg(long, default_value = "false", conflicts_with = "mids")]
    pub all: bool,

    /// Gambit results directory
    #[arg(long, default_value = crate::DEFAULT_GAMBIT_OUTPUT_DIRECTORY)]
    pub mutation_directory: String,
}
