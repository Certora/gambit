use clap::Parser;
use serde::{Deserialize, Serialize};

static DEFAULT_NO_EXPORT_MUTANTS: bool = false;
static DEFAULT_NO_OVERWRITE: bool = false;
static DEFAULT_RANDOM_SEED: bool = false;
static DEFAULT_SEED: u64 = 0;
static DEFAULT_SKIP_VALIDATE: bool = false;
static DEFAULT_LOG_INVALID: bool = false;
static DEFAULT_SOLC_OPTIMIZE: bool = false;

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

fn default_log_invalid() -> bool {
    DEFAULT_LOG_INVALID
}

fn default_solc_optimize() -> bool {
    DEFAULT_SOLC_OPTIMIZE
}

fn default_solc() -> Option<String> {
    None
}

fn default_source_root() -> Option<String> {
    None
}

fn default_num_mutants() -> Option<usize> {
    None
}

fn default_include_paths() -> Vec<String> {
    vec![]
}

fn default_import_paths() -> Vec<String> {
    vec![]
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
#[command(rename_all = "snake_case")]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct MutateParams {
    /// Json file with config
    #[arg(long, short, conflicts_with = "filename")]
    pub json: Option<String>,

    /// The name of the file to mutate.
    #[arg(conflicts_with = "json")]
    pub filename: Option<String>,

    /// If specified, randomly downsamples the number of mutants
    #[arg(long, short, default_value = None, conflicts_with = "json")]
    #[serde(default = "default_num_mutants")]
    pub num_mutants: Option<usize>,

    /// Use a random seed instead of the specified seed. This will override any
    /// value passed in with the `--seed` flag
    #[arg(long, default_value = "false", conflicts_with = "json")]
    #[serde(default = "default_random_seed")]
    pub random_seed: bool,

    /// Specify a seed for randomized down sampling. By default seed=0 is used
    /// and is deterministic, but nondeterminism can be enabled with the
    /// `--random-seed` flag
    #[arg(long, short, default_value = "0", conflicts_with = "json")]
    #[serde(default = "default_seed")]
    pub seed: u64,

    /// Output directory to place results of mutation
    #[arg(long, short)]
    pub outdir: Option<String>,

    /// Root of all source files, this determines all path offsets. By default
    /// it is the current working directory. All filenames (either specified by
    /// the --filename flag or as a "filename" field in a JSON configuration
    /// file) must exist inside the sourceroot directory.
    #[arg(long, default_value = None, hide = true, conflicts_with = "json")]
    #[serde(default = "default_source_root")]
    pub sourceroot: Option<String>,

    /// Specify the mutation operators
    #[arg(long, num_args(0..), conflicts_with = "json")]
    pub mutations: Option<Vec<String>>,

    /// Specify  _fallback mutation operators_. These operators are not applied
    /// to a program point unless all other operators fail. Fallback expression
    /// mutations are only applied to certain program points. For instance,
    /// in the expression `a + b + c`, a fallback expression mutation such as
    /// EVR will only be applied to the full expression, and not to any
    /// subexpressions, and only if no mutants were generated for `a + b + c` or
    /// its subexpressions
    #[arg(long, num_args(0..), conflicts_with = "json")]
    pub fallback_mutations: Option<Vec<String>>,

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
    pub solc: Option<String>,

    /// Run solc with the `--optimize` flag
    #[arg(long, default_value = "false", conflicts_with = "json")]
    #[serde(default = "default_solc_optimize")]
    pub solc_optimize: bool,

    /// Specify function names to mutate
    #[arg(long, num_args(1..), conflicts_with = "json")]
    pub functions: Option<Vec<String>>,

    /// Specify a contract to mutate
    #[arg(long, conflicts_with = "json")]
    pub contract: Option<String>,

    /// Specify a directory to search for solidity files during import
    #[arg(long, num_args(1..), conflicts_with = "json")]
    #[serde(default = "default_import_paths")]
    pub import_paths: Vec<String>,

    /// Map directory to search for solidity files [format: map=path]
    #[arg(long, num_args(1..), conflicts_with = "json")]
    #[serde(default = "default_import_paths")]
    pub import_maps: Vec<String>,

    /// Deprecated: Basepath argument to solc (`--base-path`)
    #[arg(long, hide = true, conflicts_with = "json")]
    pub solc_base_path: Option<String>,

    /// Deprecated: Include paths argument to solc (`--include-paths`)
    #[arg(long = "solc_include_path", hide = true, conflicts_with = "json")]
    #[serde(default = "default_include_paths")]
    pub solc_include_paths: Vec<String>,

    /// Allowpath argument to solc used during validation
    #[arg(long, conflicts_with = "json")]
    pub solc_allow_paths: Option<Vec<String>>,

    /// Solidity remappings
    #[arg(long, hide = true, num_args(1..), conflicts_with = "json")]
    pub solc_remappings: Option<Vec<String>>,

    /// Do not validate mutants by invoking solc
    #[arg(long, default_value = "false")]
    #[serde(default = "default_skip_validate")]
    pub skip_validate: bool,

    /// Log any invalid mutations that are encountered during mutant validation
    #[arg(long, default_value = "false")]
    #[serde(default = "default_log_invalid")]
    pub log_invalid: bool,

    /// Manually specify any solc arguments. These will bypass any other args
    /// passed to solc (e.g., --solc_include_paths). This is meant as a backup
    /// method in case the normal Gambit CLI does not provide the needed
    /// flexibility
    #[arg(long, default_value=None, conflicts_with = "json")]
    pub solc_raw_args: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GambitConfigFile {
    pub configurations: Vec<MutateParams>,
}

#[derive(Parser)]
#[clap(rename_all = "kebab-case")]
pub enum Command {
    Mutate(Box<MutateParams>), // Maybe we want to do other things in the future like support checking mutants?
    Summary(SummaryParams),
    /// Print the current Gambit version number
    Version,
}

/// Summarize mutants generated by a Gambit run.
///
/// By default, high level statistics are reported. Use the `--mids` flag to
/// summarize a specific list of mutant ids, and use the `--all` flag to
/// summarize all mutant ids.
#[derive(Debug, Clone, Parser, Deserialize, Serialize)]
#[command(rename_all = "kebab-case")]
pub struct SummaryParams {
    /// Print summaries of the specified mutant IDs (these IDs correspond to the
    /// "id" field in `gambit_results.json`). Multiple MIDs can be specified.
    #[arg(long, short='M', default_value = None, num_args(0..), conflicts_with = "all_mids")]
    pub mids: Option<Vec<String>>,

    /// Print a summary of all MIDs
    #[arg(long = "all", short = 'a', conflicts_with = "mids")]
    pub print_all_mids: bool,

    /// Print a short version of each mutant
    #[arg(long, short = 's')]
    pub short: bool,

    /// Gambit results directory
    #[arg(long, short='D', default_value = crate::DEFAULT_GAMBIT_OUTPUT_DIRECTORY)]
    pub mutation_directory: String,
}
