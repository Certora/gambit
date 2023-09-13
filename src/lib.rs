use std::{collections::HashMap, fs, path::PathBuf, time::Instant};

mod cli;
pub use cli::*;

mod compile;
pub use compile::*;

mod filter;
use csv::Writer;
pub use filter::*;

mod mutation;
pub use mutation::*;

mod mutant_writer;
pub use mutant_writer::*;

mod mutator;
pub use mutator::*;

mod summary;
pub use summary::*;

mod test_util;
pub use test_util::*;

mod util;
pub use util::*;

/// Execute the `mutate` command and return a mapping from output directories
/// to generated mutants.
///
/// `gambit mutate` runs on a vector of mutate parameters. Each mutate parameter
/// specifies an output directory. Parameters with the same output directory are
/// grouped and run together, and will have unique mutant ids between them.
/// Mutant ids may be shared between mutants with different output directories.
pub fn run_mutate(
    mutate_params: Vec<MutateParams>,
) -> Result<HashMap<String, Vec<Mutant>>, Box<dyn std::error::Error>> {
    log::info!("Running Gambit Mutate command");
    log::debug!("Mutate parameters: {:#?}", mutate_params);

    let start = Instant::now();
    let mut mutants_by_out_dir: HashMap<String, Vec<(Mutant, bool)>> = HashMap::default();

    let mut outdir_map: HashMap<String, Vec<MutateParams>> = HashMap::new();

    // Group mutants by outdir
    for params in mutate_params {
        let outdir = &params.outdir;
        outdir_map
            .entry(outdir.clone().unwrap_or(default_gambit_output_directory()))
            .or_insert(vec![])
            .push(params);
    }

    let mut total_num_mutants = 0;
    // Iterate through each out dir and its associated parameters and generate mutants
    for (outdir, outdir_params) in outdir_map.iter() {
        /*                                                          *
         *               SETUP OUTPUT DIRECTORY                     *
         *               ======================                     */

        // Iterate through the params for this outdir to discover if we are
        // not overwriting.
        //
        // NOTE: This is a temporary way to handle the fact that overwriting
        // is a property of an output directory, but is specified as a property
        // of a particular filename we are mutating. The configuration file
        // should eventually be updated to reflect this structural difference, but for now we are adopting the convention that:
        //
        // Whenever one parameter targetting a given output directory specifies
        // that an output directory should not be overwritten, all parameters
        // targetting that directory are marked as no_output
        let no_overwrite = outdir_params.iter().any(|p| p.no_overwrite);

        let outdir_path = PathBuf::from(outdir);

        if outdir_path.exists() {
            if !no_overwrite {
                if fs::metadata(outdir_path.as_path()).is_ok() {
                    match fs::remove_dir_all(&outdir_path) {
                        Ok(_) => log::info!("Removed outdir {}", outdir_path.display()),
                        Err(_) => log::info!(
                            "Couldn't remove outdir {} even though it exists",
                            outdir_path.display()
                        ),
                    }
                }
            } else {
                eprintln!(
                    "[!] Output directory {} exists! You can:",
                    outdir_path.display()
                );
                eprintln!("  (1) Manually remove {}", outdir_path.display());
                eprintln!(
                    "  (2) Use the `--overwrite` flag to skip this message and overwrite {}",
                    outdir_path.display()
                );
                eprintln!("  (3) Specify another output directory with `--outdir OUTPUT_LOCATION`");
                continue;
            }
        }
        log::info!("Creating outdir {}", outdir_path.display());
        fs::create_dir_all(&outdir_path)?;

        // Now, let's get to the fun stuff! Iterate through the parameters, and for each:
        // 1. generate mutants
        // 2. filter the mutants (if num-mutants was specified)
        // 3. optionally validate the mutants
        for params in outdir_params.iter() {
            log::info!("Processing params: {:?}", params);
            let export = !params.no_export;

            /*                                          *
             *               MUTATE                     *
             *               ======                     */
            log::info!("Creating mutator");
            let mut mutator = Mutator::from(params);
            let sources = params
                .filename
                .iter()
                .map(|x| x.clone())
                .collect::<Vec<String>>();
            let mutants = mutator.mutate(sources)?.clone();
            log::info!(
                "(pre filter/validate) Generated {} mutants for {}",
                &mutants.len(),
                params.filename.as_ref().unwrap()
            );

            /*                                                   *
             *               FILTER/VALIDATE                     *
             *               ===============                     */
            let mut solc = Solc::new(
                params.solc.clone().unwrap_or_else(|| "solc".to_string()),
                outdir_path.clone(),
            );
            solc.with_vfs_roots_from_params(params);
            let mut validator = Validator { solc };
            log::debug!("Validator: {:?}", validator);
            let (sampled, invalid) = if let Some(num_mutants) = params.num_mutants {
                log::info!("Filtering down to {} mutants", num_mutants);
                log::debug!("  seed: {:?}", params.seed);
                log::debug!("  validating?: {}", !params.skip_validate);
                let seed = if params.random_seed {
                    None
                } else {
                    Some(params.seed)
                };
                let mut filter =
                    RandomDownSampleFilter::new(seed, !params.skip_validate, validator);
                let (sampled, invalid) = filter.filter_mutants(&mutator, num_mutants)?;
                if !params.skip_validate {
                    log::info!(
                        "Filtering and Validation resulted in {} valid mutants",
                        sampled.len()
                    );
                    log::info!("   and {} invalid mutants", invalid.len());
                } else {
                    log::info!("Filtering resulted in {} mutants", sampled.len());
                }
                (sampled, invalid)
            } else if params.skip_validate {
                log::info!("Skipping validation");
                (mutants, vec![])
            } else {
                let (sampled, invalid) = validator.get_valid_mutants(&mutants);
                log::info!("Validation resulted in {} valid mutants", sampled.len());
                log::info!("   and {} invalid mutants", invalid.len());
                (sampled, invalid)
            };

            if params.log_invalid {
                let invalid_log = &outdir_path.join("invalid.log");
                let mut w = Writer::from_path(invalid_log)?;
                for (i, mutant) in invalid.iter().enumerate() {
                    let (line_no, col_no) = mutant.get_line_column();
                    let mid = i + 1;
                    let line_col = format!("{}:{}", line_no, col_no);
                    w.write_record([
                        mid.to_string().as_str(),
                        mutant.op.short_name().as_str(),
                        mutant.vfs_path().to_str().unwrap(),
                        line_col.as_str(),
                        mutant.orig.as_str(),
                        mutant.repl.as_str(),
                    ])?;
                }
            }

            total_num_mutants += sampled.len();
            log::info!("Adding {} mutants to global mutant pool", sampled.len());

            let mut sampled: Vec<(Mutant, bool)> =
                sampled.iter().map(|m| (m.clone(), export)).collect();

            if !mutants_by_out_dir.contains_key(outdir) {
                mutants_by_out_dir.insert(outdir.clone(), vec![]);
            }
            let ms: &mut Vec<(Mutant, bool)> = mutants_by_out_dir.get_mut(outdir).unwrap();
            ms.append(&mut sampled);
        }
    }

    let mut results: HashMap<String, Vec<Mutant>> = HashMap::default();

    /*                                                 *
     *               WRITE MUTANTS                     *
     *               =============                     */
    for (outdir, mutants) in mutants_by_out_dir {
        log::info!("Writing mutants for output directory {}", &outdir);
        results.insert(
            outdir.clone(),
            mutants.iter().map(|(m, _)| m).cloned().collect(),
        );
        MutantWriter::new(outdir).write_mutants(&mutants)?;
    }

    let t = start.elapsed().as_secs_f64();
    println!(
        "Generated {} mutants in {:.2} seconds",
        total_num_mutants, t
    );
    log::info!("Generated {} mutants in {}", total_num_mutants, t);
    Ok(results)
}

pub fn run_summary(params: SummaryParams) -> Result<(), Box<dyn std::error::Error>> {
    log::info!("Running Gambit Summary");
    log::debug!("Summary parameters: {:?}", params);
    summarize(params)
}

pub fn print_version() {
    println!("Gambit version: {}", env!("CARGO_PKG_VERSION"))
}
