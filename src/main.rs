use std::path::{Path, PathBuf};

use clap::Parser;
use gambit::{normalize_path, repair_remapping, run_mutate, run_summary, Command, MutateParams};

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
                // Run from config file
                let json_contents = std::fs::read_to_string(json_path)?;
                let json: serde_json::Value = serde_json::from_reader(json_contents.as_bytes())?;
                log::info!("Read configuration json: {:#?}", json);

                let mut mutate_params: Vec<MutateParams> = if json.is_array() {
                    match serde_json::from_str(&json_contents) {
                        Ok(xs) => xs,
                        Err(msg) => {
                            println!("{}", &msg);
                            std::process::exit(1);
                        }
                    }
                } else if json.is_object() {
                    let single_param: MutateParams = match serde_json::from_str(&json_contents) {
                        Ok(xs) => xs,
                        Err(msg) => {
                            println!("{}", &msg);
                            std::process::exit(1);
                        }
                    };
                    vec![single_param]
                } else {
                    panic!("Invalid configuration file: must be an array or an object")
                };
                log::debug!("Deserialized JSON into MutateParams: {:#?}", &mutate_params);

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
                // parameter's `filename` value is prefixed by (or belongs to)
                // it's source root.
                //
                // NOTE: not all files need to belong to `sourceroot`! Only the
                // `param.filename`, since this is written to logs. In
                // particular, compiler arguments (e.g., specified by the
                // `--solc-allowpaths` flag) will not be checked for inclusion
                // in `sourceroot`.
                let config_pb = PathBuf::from(&json_path);
                log::info!("config: {}", config_pb.display());
                let config_pb = config_pb.canonicalize()?;
                log::info!("canonical config: {}", config_pb.display());
                let config_parent_pb = config_pb.parent().unwrap();
                log::info!("config parent: {}", config_parent_pb.display());
                let json_parent_directory = config_parent_pb.canonicalize()?;

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
                    //
                    // We need to check for the following filenames to resolve
                    // with respect to the config file's parent directory:
                    //
                    // | Parameter       | Resolve WRT Conf? | Sourceroot Inclusion? |
                    // | --------------- | ----------------- | --------------------- |
                    // | filename        | Yes               | Yes                   |
                    // | outdir          | Yes               | No                    |
                    // | solc_allowpaths | Yes               | No                    |
                    // | solc_basepath   | Yes               | No                    |
                    // | solc_remapping  | Yes               | No                    |
                    log::info!("    Performing Filename Resolution");

                    // PARAM: Filename
                    log::info!("    [.] Resolving params.filename");
                    let filename_path: PathBuf = match params.filename.clone() {
                        Some(filename) => {
                            resolve_config_file_path(&filename, &json_parent_directory)?
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

                    // PARAM: Outdir
                    // We can't use `resolve_config_file_path` because it might
                    // not exist yet, so we can't canonicalize

                    log::info!("    [.] Resolving params.outdir");
                    let outdir_path = PathBuf::from(&params.outdir);
                    let outdir_path = if outdir_path.is_absolute() {
                        normalize_path(&outdir_path)
                    } else {
                        normalize_path(&json_parent_directory.join(&outdir_path))
                    };
                    let outdir = outdir_path.to_str().unwrap().to_string();
                    log::info!(
                        "    [->] Resolved path `{}` to `{}`",
                        &params.outdir,
                        &outdir,
                    );

                    // PARAM: solc_allowpaths
                    log::info!("    [.] Resolving params.allow_paths");
                    let allow_paths = if let Some(allow_paths) = &params.solc_allow_paths {
                        Some(resolve_config_file_paths(
                            allow_paths,
                            &json_parent_directory,
                        )?)
                    } else {
                        None
                    };

                    // PARAM: solc_basepath
                    log::info!("    [.] Resolving params.solc_basepath");
                    let basepath = if let Some(basepaths) = &params.solc_base_path {
                        Some(resolve_config_file_path(basepaths, &json_parent_directory)?)
                            .map(|bp| bp.to_str().unwrap().to_string())
                    } else {
                        None
                    };

                    // PARAM: solc_remappings
                    log::info!("    [.] Resolving params.solc_remapping");
                    let remapping: Option<Vec<String>> =
                        params.solc_remappings.as_ref().map(|remapping| {
                            remapping
                                .iter()
                                .map(|rm| {
                                    repair_remapping(
                                        rm.as_str(),
                                        Some(json_parent_directory.to_str().unwrap()),
                                    )
                                })
                                .collect()
                        });

                    // Finally, update params with resolved source root and filename.
                    // (We don't update earlier to preserve the state of params
                    // for error reporting: reporting the parsed in value of
                    // `params` will be more helpful to the end user than
                    // reporting the modified value of params).
                    params.sourceroot = Some(source_root_string.clone());
                    params.filename = Some(filename_string.clone());
                    params.outdir = outdir;
                    params.solc_allow_paths = allow_paths;
                    params.solc_base_path = basepath;
                    params.solc_remappings = remapping;
                }
                run_mutate(mutate_params)?;
            } else {
                log::debug!("Running CLI MutateParams: {:#?}", &params);
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
                //
                // We need to canonicalize the following files, possibly
                // checking for sourceroot inclusion.
                //
                // | Parameter       | Sourceroot Inclusion? |
                // | --------------- | --------------------- |
                // | filename        | Yes                   |
                // | outdir          | No                    |
                // | solc_allowpaths | No                    |
                // | solc_basepath   | No                    |
                // | solc_remapping  | No                    |
                log::info!("    Performing Filename Resolution");

                log::info!("    [.] Resolving params.filename");
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

                log::info!("    [.] Resolving params.outdir {}", &params.outdir);
                let outdir = normalize_path(&PathBuf::from(&params.outdir))
                    .to_str()
                    .unwrap()
                    .to_string();

                log::info!("    [.] Resolving params.solc_allowpaths");
                let solc_allowpaths = params.solc_allow_paths.map(|aps| {
                    aps.iter()
                        .map(|p| {
                            PathBuf::from(p)
                                .canonicalize()
                                .unwrap()
                                .to_str()
                                .unwrap()
                                .to_string()
                        })
                        .collect()
                });

                log::info!("    [.] Resolving params.solc_basepath");
                let solc_basepath = params.solc_base_path.map(|bp| {
                    PathBuf::from(bp)
                        .canonicalize()
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .to_string()
                });

                log::info!("    [.] Resolving params.solc_remapping");
                let solc_remapping = params.solc_remappings.as_ref().map(|rms| {
                    rms.iter()
                        .map(|rm| repair_remapping(rm.as_str(), None))
                        .collect()
                });
                log::info!(
                    "    [->] Resolved solc-remapping:\n    {:#?} to \n    {:#?}",
                    &params.solc_remappings,
                    &solc_remapping
                );

                // Finally, update params with resolved source root and filename.
                // (We don't update earlier to preserve the state of params
                // for error reporting: reporting the parsed in value of
                // `params` will be more helpful to the end user than
                // reporting the modified value of params).
                params.sourceroot = Some(source_root_string);
                params.filename = Some(filename_string);
                params.outdir = outdir;
                params.solc_allow_paths = solc_allowpaths;
                params.solc_base_path = solc_basepath;
                params.solc_remappings = solc_remapping;

                run_mutate(vec![*params])?;
            }
        }
        Command::Summary(params) => {
            run_summary(params)?;
        }
    }
    Ok(())
}

/// Resolve a filename with respect to the directory containing the config file
fn resolve_config_file_path(
    path: &String,
    json_parent_directory: &Path,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let path = PathBuf::from(&path);
    let result = if path.is_absolute() {
        path.canonicalize()?
    } else {
        json_parent_directory.join(&path).canonicalize()?
    };
    log::info!(
        "    [->] Resolved path `{}` to `{}`",
        path.display(),
        result.display()
    );
    Ok(result)
}

fn resolve_config_file_paths(
    paths: &Vec<String>,
    json_parent_directory: &Path,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut result: Vec<String> = vec![];
    for path in paths {
        result.push(
            resolve_config_file_path(path, json_parent_directory)?
                .to_str()
                .unwrap()
                .to_string(),
        );
    }
    Ok(result)
}
