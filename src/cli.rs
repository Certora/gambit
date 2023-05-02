use clap::Parser;
use serde::{Deserialize, Serialize};

static DEFAULT_NO_EXPORT_MUTANTS: bool = false;
static DEFAULT_NO_OVERWRITE: bool = false;
static DEFAULT_RANDOM_SEED: bool = false;
static DEFAULT_SEED: u64 = 0;
static DEFAULT_SKIP_VALIDATE: bool = false;
static DEFAULT_SOLC_OPTIMIZE: bool = false;
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

fn default_solc_optimize() -> bool {
    DEFAULT_SOLC_OPTIMIZE
}

fn default_solc() -> String {
    DEFAULT_SOLC.to_string()
}

fn default_source_root() -> Option<String> {
    None
}

fn default_num_mutants() -> Option<usize> {
    None
}

/// Mutate solidity code.
///
/// The `mutate` command requires either a `--filename` or a `--json`
/// configuration file to be passed, and these are mutually exclusive.
///
/// # Examples
/// 1. `gambit mutate --filename path/to/file.sol` this will apply all mutations to file.sol.
///
/// 2. `gambit mutate --json path/to/config.json`: this runs mutations specified
///    in the configuration file
///
/// Only one filename can be specified from command line at a time, but multiple
/// files can be specified in a configuration.
#[derive(Debug, Clone, Parser, Deserialize, Serialize)]
#[command(rename_all = "kebab-case")]
#[serde(rename_all = "kebab-case")]
pub struct MutateParams {
    /// Json file with config
    #[arg(long, short, conflicts_with = "filename")]
    pub json: Option<String>,

    /// The name of the file to mutate. Note that this filename must be a
    /// descendent of the source root (`.` by default, or specified by the
    /// `--sourceroot` flag).
    ///
    /// # Example
    ///
    /// Running:
    ///
    /// `gambit mutate --filename /path/to/file.sol --sourceroot /some/other/path`
    ///
    /// will cause an error.  This is because `/path/to/file.sol` is not
    /// (recursively) contained in `/some/other/path`. On the other hand, if
    /// our working directory is `/path/to`, running:
    ///
    /// `gambit mutate --filename /path/to/file.sol`
    ///
    /// will work because `--sourceroot` is by default `.` which, in this case,
    /// expands to `/path/to`, which contains `file.sol`.
    #[arg(long, short, conflicts_with = "json")]
    pub filename: Option<String>,

    /// If specified, randomly downsamples the number of mutants
    #[arg(long, short, default_value = None)]
    #[serde(default = "default_num_mutants")]
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

    /// Root of all source files, this determines all path offsets. By default
    /// it is the current working directory. All filenames (either specified by
    /// the --filename flag or as a "filename" field in a JSON configuration
    /// file) must exist inside the sourceroot directory.
    #[arg(long, default_value = None)]
    #[serde(default = "default_source_root")]
    pub sourceroot: Option<String>,

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

    /// Run solc with the `--optimize` flag
    #[arg(long, default_value = "false")]
    #[serde(default = "default_solc_optimize")]
    pub solc_optimize: bool,

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

/// Summarize mutants generated by a Gambit run. By default, all mutant ids are
/// summarized. Use the `--mids` flag to specify a list of mids to summarize
#[derive(Debug, Clone, Parser, Deserialize, Serialize)]
#[command(rename_all = "kebab-case")]
pub struct SummaryParams {
    /// Print summaries of the specified mutant IDs (these IDs correspond to the
    /// "id" field in `gambit_results.json`). Multiple MIDs can be specified.
    /// If `--all` is specified, this is ignored.
    #[arg(long, default_value = None, num_args(0..))]
    pub mids: Option<Vec<String>>,

    /// Gambit results directory
    #[arg(long, default_value = crate::DEFAULT_GAMBIT_OUTPUT_DIRECTORY)]
    pub mutation_directory: String,
}
