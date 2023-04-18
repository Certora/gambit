use std::path::PathBuf;

use clap::Parser;
use gambit::{run_mutate, run_summary, Command, MutateParams};

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
