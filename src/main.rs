use std::{collections::HashMap, fs, path::PathBuf, time::Instant};

use clap::Parser;
use gambit::{
    summarize, Command, Mutant, MutantFilter, MutantWriter, MutateParams, Mutator,
    RandomDownSampleFilter, SummaryParams,
};

/// Entry point
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = env_logger::builder().try_init();
    match Command::parse() {
        Command::Mutate(params) => {
            // The user has specified a configuration file.
            //
            // Configuration files have two forms: (1) a JSON array of JSON
            // objects, where each object represents a `MutateParams` struct,
            // and (2) a single JSON object representing a `MutateParams`
            // struct. The second case is syntactic sugar for an array with a
            // single object.
            //
            // To tell the difference we deserialzie as a `serde_json::Value`
            // and check if it's an array or an object and create a
            // `Vec<MutateParams>` based on this.
            if let Some(json_path) = &params.json {
                log::info!("Running from configuration");
                // Run fron config file
                let json_contents = std::fs::read_to_string(&json_path)?;
                let json: serde_json::Value = serde_json::from_reader(json_contents.as_bytes())?;

                let mut mutate_params: Vec<MutateParams> = if json.is_array() {
                    serde_json::from_str(&json_contents)?
                } else if json.is_object() {
                    let single_param: MutateParams = serde_json::from_str(&json_contents)?;
                    vec![single_param]
                } else {
                    panic!("Invalid configuration file: must be an array or an object")
                };

                // We also have to include some path update logic: a config file
                // uses paths relative to the parent directory of the config file.
                // This may be different than the current working directory, so we
                // need to compute paths as offsets from the config's parent
                // directory.
                let pb = PathBuf::from(&json_path);
                let json_parent_directory = pb.parent().unwrap();

                for params in mutate_params.iter_mut() {
                    log::info!("Resolving filename: {}", params.filename.as_ref().unwrap());
                    params.filename = params.filename.clone().map(|fnm| {
                        json_parent_directory
                            .join(fnm)
                            .to_str()
                            .unwrap()
                            .to_string()
                    });
                    log::info!("[ -> ] Resolved to: {}", &params.filename.as_ref().unwrap());
                }
                run_mutate(mutate_params)?;
            } else {
                run_mutate(vec![params])?;
            }
        }
        Command::Summary(params) => {
            run_summary(params)?;
        }
    }
    Ok(())
}

/// Execute the `mutate` command
fn run_mutate(mutate_params: Vec<MutateParams>) -> Result<(), Box<dyn std::error::Error>> {
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
        let overwrite = outdir_params.iter().any(|p| p.overwrite);
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
            let export = params.export_mutants;

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
                let filter = RandomDownSampleFilter::new(params.seed, !params.skip_validate);
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

    for (outdir, mutants) in mutants_by_out_dir {
        log::info!("Writing mutants for output directory {}", &outdir);
        MutantWriter::new(outdir).write_mutants(&mutants)?;
    }

    let t = start.elapsed().as_secs_f64();
    println!(
        "Generated {} mutants in {:.2} seconds",
        total_num_mutants, t
    );
    log::info!("Generated {} mutants in {}", total_num_mutants, t);
    Ok(())
}

fn run_summary(params: SummaryParams) -> Result<(), Box<dyn std::error::Error>> {
    log::info!("Running Gambit Summary");
    log::debug!("Summary parameters: {:?}", params);
    summarize(params)
}
