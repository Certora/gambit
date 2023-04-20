mod ast;
use std::{collections::HashMap, fs, path::PathBuf, time::Instant};

pub use ast::*;

mod cli;
pub use cli::*;

mod compile;
pub use compile::*;

mod filter;
pub use filter::*;

mod mutation;
pub use mutation::*;

mod mutant_writer;
pub use mutant_writer::*;

mod mutator;
pub use mutator::*;

mod source;
pub use source::*;

mod summary;
pub use summary::*;

mod test_util;
pub use test_util::*;

mod util;
pub use util::*;

/// Execute the `mutate` command. This returns a mapping from output directories
/// to generated mutants.
pub fn run_mutate(
    mutate_params: Vec<MutateParams>,
) -> Result<HashMap<String, Vec<Mutant>>, Box<dyn std::error::Error>> {
    log::info!("Running Gambit Mutate command");
    log::debug!("Mutate parameters: {:?}", mutate_params);

    let start = Instant::now();
    let mut mutants_by_out_dir: HashMap<String, Vec<(Mutant, bool)>> = HashMap::default();

    let mut outdir_map: HashMap<String, Vec<MutateParams>> = HashMap::new();

    // Group mutants by outdir
    for params in mutate_params {
        let outdir = &params.outdir;
        outdir_map
            .entry(outdir.to_string())
            .or_insert(vec![])
            .push(params);
    }

    let mut total_num_mutants = 0;
    // Iterate through each out dir and its associated parameters and generate mutants
    for (outdir, outdir_params) in outdir_map.iter() {
        // Iterate through the params for this outdir to discover if we are
        // overwriting
        let no_overwrite = outdir_params.iter().any(|p| p.no_overwrite);
        let overwrite = !no_overwrite;
        let outdir_path = PathBuf::from(outdir);
        if outdir_path.exists() {
            if overwrite {
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
                eprintln!("");
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

        // Now, iterate through these parameters and generate mutants
        for params in outdir_params.iter() {
            log::info!("Processing params: {:?}", params);
            let export = !params.no_export;

            log::info!("Creating mutator");
            let mut mutator = Mutator::from(params);
            log::info!("Generating mutants");
            let mutants = mutator.mutate()?.clone();
            log::info!(
                "(pre filter/validate) Generated {} mutants for {}",
                &mutants.len(),
                params.filename.as_ref().unwrap()
            );

            // Check if we are filtering
            let mutants = if let Some(num_mutants) = params.num_mutants {
                log::info!("Filtering down to {} mutants", num_mutants);
                log::debug!("  seed: {:?}", params.seed);
                log::debug!("  validating?: {}", !params.skip_validate);
                let seed = if params.random_seed {
                    None
                } else {
                    Some(params.seed)
                };
                let filter = RandomDownSampleFilter::new(seed, !params.skip_validate);
                let mutants = filter.filter_mutants(&mutator, num_mutants)?;
                log::info!("Filtering resulted in {} mutants", mutants.len());
                mutants
            } else {
                if params.skip_validate {
                    log::info!("Skipping validation");
                    mutants
                } else {
                    let mutants = mutator.get_valid_mutants(&mutants);
                    log::info!("Validation resulted in {} mutants", mutants.len());
                    mutants
                }
            };
            total_num_mutants += mutants.len();
            log::info!("Adding {} mutants to global mutant pool", mutants.len());

            let mut mutants: Vec<(Mutant, bool)> =
                mutants.iter().map(|m| (m.clone(), export)).collect();

            if !mutants_by_out_dir.contains_key(outdir) {
                mutants_by_out_dir.insert(outdir.clone(), vec![]);
            }
            let ms: &mut Vec<(Mutant, bool)> = mutants_by_out_dir.get_mut(outdir).unwrap();
            ms.append(&mut mutants);
        }
    }

    let mut results: HashMap<String, Vec<Mutant>> = HashMap::default();

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
