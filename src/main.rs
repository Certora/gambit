use std::path::PathBuf;

use clap::Parser;
use gambit::{run_mutate, run_summary, Command, MutateParams};

/// Entry point
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = env_logger::builder().try_init();
    match Command::parse() {
        Command::Mutate(mut params) => {
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

                // # Path Resolutions in Configuration Files
                //
                // All paths specified _in a configuration file_ are relative to
                // the parent directory of the configuration file. This means
                // that they will always resolve to the same canonical path,
                // regardless of which directory Gambit is called from.
                //
                // ## Source Root Resolution
                //
                // The _source root_ describes where the conceptual root of the
                // file source tree is. This is used for outputting relative
                // paths when Gambit reports on a mutation run and exports
                // mutants to disk.
                //
                // If a "sourceroot" field is provided in the configration file,
                // Gambit resolves it according to the following rules:
                //
                // 1. A **relative** source root path is resolved with respect to
                //    the parent directory of the configuration file and
                //    canonicalized.
                //
                // 2. An **absolute** source root path resolves to itself
                //
                // If no source root is provided in the configuration file,
                // Gambit uses the current working directory as the source root.
                //
                // ## Filename Resolution
                //
                // After Source Root Resolution is performed, Gambit performs
                // Filename Resolution. First, Gambit resolves each filename
                // according to the following rules:
                //
                // 1. A **relative** filename is resolved with respect to the
                //    parent directory of the configuration file.
                //
                // 2. An **absolute** filename resolves to itself
                //
                // After Filename Resolution, Gambit ensures that each
                // parameter's filename is prefixed by (or belongs to) it's
                // source root.
                let pb = PathBuf::from(&json_path);
                let json_parent_directory = pb.parent().unwrap();

                log::info!("Performing Path Resolution for Configurations");
                log::info!("Found {} configurations", mutate_params.len());

                for (i, params) in mutate_params.iter_mut().enumerate() {
                    // Source Root Resolution
                    log::info!("Configuration {}", i + 1);
                    log::info!("    Performing Source Root Resolution");
                    let source_root_path: PathBuf = match params.sourceroot.clone() {
                        Some(sr) => {
                            let raw_source_root_path = PathBuf::from(&sr);
                            let resolved_source_root_path = if raw_source_root_path.is_absolute() {
                                raw_source_root_path.canonicalize()?
                            } else {
                                json_parent_directory
                                    .join(raw_source_root_path)
                                    .canonicalize()?
                            };
                            log::info!(
                                "    [->] Resolved sourceroot `{}` to `{}`",
                                &sr,
                                resolved_source_root_path.display()
                            );
                            resolved_source_root_path
                        }
                        None => {
                            let resolved_source_root_path = PathBuf::from(".").canonicalize()?;
                            log::info!("    No sourceroot provided in configration");
                            log::info!(
                                "    [->] Resolved sourceroot to current working directory `{}`",
                                resolved_source_root_path.display()
                            );
                            resolved_source_root_path
                        }
                    };
                    let source_root_string = source_root_path.to_str().unwrap().to_string();

                    // Filename Resolution
                    log::info!("    Performing Filename Resolution");

                    let filename_path: PathBuf = match params.filename.clone() {
                        Some(filename) => {
                            let raw_filename_path = PathBuf::from(&filename);
                            let resolved_filename_path = if raw_filename_path.is_absolute() {
                                raw_filename_path.canonicalize()?
                            } else {
                                json_parent_directory
                                    .join(raw_filename_path)
                                    .canonicalize()?
                            };
                            log::info!(
                                "    [->] Resolved filename `{}` to `{}`",
                                &filename,
                                resolved_filename_path.display()
                            );
                            resolved_filename_path
                        }
                        None => {
                            log::error!("[!!] Found a configuration without a filename!");
                            log::error!("[!!] Parameters: {:#?}", params);
                            log::error!("[!!] Exiting.");
                            // TODO: Replace exit with an error
                            std::process::exit(1);
                        }
                    };
                    let filename_string = filename_path.to_str().unwrap().to_string();

                    // Check that filename is a member of sourceroot
                    if !filename_path.starts_with(&source_root_path) {
                        log::error!( "[!!] Illegal Configuration: Resolved filename `{}` is not prefixed by the derived sourceroot {}",
                            &filename_string,
                            &source_root_string,
                        );
                        log::error!("[!!] Parameters:\n{:#?}", params);
                        log::error!("[!!] Exiting.");
                        // TODO: Replace exit with an error
                        std::process::exit(1);
                    }
                    log::info!(
                        "    [->] Resolved filename `{}` belongs to sourceroot `{}`",
                        &filename_string,
                        &source_root_string
                    );

                    // Finally, update params with resolved source root and filename.
                    // (We don't update earlier to preserve the state of params
                    // for error reporting: reporting the parsed in value of
                    // `params` will be more helpful to the end user than
                    // reporting the modified value of params).
                    params.sourceroot = Some(source_root_string.clone());
                    params.filename = Some(filename_string.clone());
                }
                run_mutate(mutate_params)?;
            } else {
                // # Path Resolution for CLI Provided Parameters
                //
                // All relative paths specified _from the CLI_ are relative to
                // the current working directory. This means that relative paths
                // will resolve to different canonical paths depending on where
                // they are called from.
                //
                // ## Source Root Resolution
                //
                // The _source root_ describes where the conceptual root of the
                // file source tree is. This is used for outputting relative
                // paths when Gambit reports on a mutation run and exports
                // mutants to disk.
                //
                // If the `--sourceroot` parameter is provided by the user,
                // Gambit resolves it according to the following rules:
                //
                // 1. A **relative** source root path is resolved with respect to
                //    the current working directory
                //
                // 2. An **absolute** source root path resolves to itself
                //
                // If no source root is provided, Gambit uses the current
                // working directory as the source root.
                //
                // ## Filename Resolution
                //
                // After Source Root Resolution is performed, Gambit performs
                // Filename Resolution. First, Gambit resolves each filename
                // according to the following rules:
                //
                // 1. A **relative** filename is resolved with respect to the
                //    current working directory.
                //
                // 2. An **absolute** filename resolves to itself
                //
                // If no filename is provided, Gambit reports an error and
                // exits.
                //
                // After Filename Resolution, Gambit ensures that each
                // parameter's filename is prefixed by (or belongs to) it's
                // source root. If not, Gambit reports an error and exits.

                // Source Root Resolution
                log::info!("Performing Path Resolution for CLI");
                log::info!("    Performing Source Root Resolution");
                let source_root_path: PathBuf = match params.sourceroot.clone() {
                    Some(sr) => {
                        let raw_source_root_path = PathBuf::from(&sr);
                        let resolved_source_root_path = raw_source_root_path.canonicalize()?;
                        log::info!(
                            "    [->] Resolved sourceroot `{}` to `{}`",
                            &sr,
                            resolved_source_root_path.display()
                        );
                        resolved_source_root_path
                    }
                    None => {
                        let resolved_source_root_path = PathBuf::from(".").canonicalize()?;
                        log::info!("    No sourceroot provided in configration");
                        log::info!(
                            "    [->] Resolved sourceroot to current working directory `{}`",
                            resolved_source_root_path.display()
                        );
                        resolved_source_root_path
                    }
                };
                let source_root_string = source_root_path.to_str().unwrap().to_string();

                // Filename Resolution
                log::info!("    Performing Filename Resolution");

                let filename_path: PathBuf = match params.filename.clone() {
                    Some(filename) => {
                        let raw_filename_path = PathBuf::from(&filename);
                        let resolved_filename_path = raw_filename_path.canonicalize()?;
                        log::info!(
                            "    [->] Resolved filename `{}` to `{}`",
                            &filename,
                            resolved_filename_path.display()
                        );
                        resolved_filename_path
                    }
                    None => {
                        log::error!("[!!] Found a configuration without a filename!");
                        log::error!("[!!] Parameters: {:#?}", params);
                        log::error!("[!!] Exiting.");
                        // TODO: Replace exit with an error
                        std::process::exit(1);
                    }
                };
                let filename_string = filename_path.to_str().unwrap().to_string();

                // Check that filename is a member of sourceroot
                if !filename_path.starts_with(&source_root_path) {
                    log::error!( "[!!] Illegal Configuration: Resolved filename `{}` is not prefixed by the derived sourceroot {}",
                            &filename_string,
                            &source_root_string,
                        );
                    log::error!("[!!] Parameters:\n{:#?}", params);
                    log::error!("[!!] Exiting.");
                    // TODO: Replace exit with an error
                    std::process::exit(1);
                }
                log::info!(
                    "    [->] Resolved filename `{}` belongs to sourceroot `{}`",
                    &filename_string,
                    &source_root_string
                );

                // Finally, update params with resolved source root and filename.
                // (We don't update earlier to preserve the state of params
                // for error reporting: reporting the parsed in value of
                // `params` will be more helpful to the end user than
                // reporting the modified value of params).
                params.sourceroot = Some(source_root_string.clone());
                params.filename = Some(filename_string.clone());

                run_mutate(vec![params])?;
            }
        }
        Command::Summary(params) => {
            run_summary(params)?;
        }
    }
    Ok(())
}
