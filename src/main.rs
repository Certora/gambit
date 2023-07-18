use std::path::{Path, PathBuf};

use clap::Parser;
use gambit::{
    default_gambit_output_directory, normalize_path, repair_remapping, run_mutate, run_summary,
    Command, MutateParams,
};

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
                    serde_json::from_str(&json_contents)?
                } else if json.is_object() {
                    vec![serde_json::from_str(&json_contents)?]
                } else {
                    panic!("Invalid configuration file: must be an array or an object")
                };
                log::debug!("Deserialized JSON into MutateParams: {:#?}", &mutate_params);

                let config_pb = PathBuf::from(&json_path);
                let config_pb = config_pb.canonicalize()?;
                let config_parent_pb = config_pb.parent().unwrap();
                let json_parent_directory = config_parent_pb.canonicalize()?;

                log::info!("config: {:?}", config_pb);
                log::info!("canonical config: {:?}", config_pb);
                log::info!("config parent: {}", config_parent_pb.display());

                log::info!("Performing Path Resolution for Configurations");
                log::info!("Found {} configurations", mutate_params.len());

                for (i, params) in mutate_params.iter_mut().enumerate() {
                    // Source Root Resolution
                    log::info!("Configuration {}", i + 1);

                    // PARAM: Filename
                    log::info!("    [.] Resolving params.filename");
                    let filename = params
                        .filename
                        .clone()
                        .expect("No filename in configuration");
                    let filepath = PathBuf::from(&filename);
                    println!("filepath: {:?}", filepath);
                    let filepath = if filepath.is_absolute() {
                        filepath
                    } else {
                        let joined = config_parent_pb.join(filepath);
                        println!("joined: {:?}", &joined);
                        joined.canonicalize().unwrap()
                    };

                    // PARAM: Outdir
                    // + If an absolute `outdir` is specified, normalize it
                    // + If a relative (non-absolute) `outdir` is specified,
                    //   normalize it with respect to the JSON parent directory
                    // + If no `outdir` is specified, use `$CWD/gambit_out`
                    //
                    // Note: we cannot use `resolve_config_file_path` because
                    // `outdir` might not exist yet

                    log::info!("    [.] Resolving params.outdir");
                    let outdir_path = match &params.outdir {
                        Some(outdir) => {
                            let outdir_path = PathBuf::from(outdir);
                            if outdir_path.is_absolute() {
                                normalize_path(&outdir_path)
                            } else {
                                normalize_path(&json_parent_directory.join(&outdir_path))
                            }
                        }
                        None => normalize_path(
                            &PathBuf::from(".").join(default_gambit_output_directory()),
                        ),
                    };
                    let outdir = outdir_path.to_str().unwrap().to_string();
                    log::info!(
                        "    [->] Resolved path `{:?}` to `{}`",
                        &params.outdir.clone(),
                        &outdir,
                    );

                    // PARAM: solc_allowpaths
                    log::info!("    [.] Resolving params.solc_allow_paths");
                    let allow_paths = if let Some(allow_paths) = &params.solc_allow_paths {
                        Some(resolve_config_file_paths(
                            allow_paths,
                            &json_parent_directory,
                        )?)
                    } else {
                        None
                    };

                    // PARAM: solc_basepath
                    log::info!("    [.] Resolving params.solc_base_path");
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
                    params.filename = Some(filepath.to_str().unwrap().to_string());
                    params.outdir = Some(outdir);
                    params.solc_allow_paths = allow_paths;
                    params.solc_base_path = basepath;
                    params.solc_remappings = remapping;
                }
                run_mutate(mutate_params)?;
            } else {
                log::debug!("Running CLI MutateParams: {:#?}", &params);

                // # Path Resolution for CLI Provided Parameters
                log::info!("    Performing Filename Resolution");
                //                let filename = params.filename.expect("No provided filename");

                log::info!("    [.] Resolving params.filename");
                let filename = params
                    .filename
                    .clone()
                    .expect("No filename in configuration");
                let filepath = PathBuf::from(&filename).canonicalize().unwrap();
                println!("filepath: {:?}", filepath);

                log::info!("    [.] Resolving params.outdir {:?}", &params.outdir);

                let outdir = normalize_path(&PathBuf::from(
                    &params.outdir.unwrap_or(default_gambit_output_directory()),
                ))
                .to_str()
                .unwrap()
                .to_string();
                log::info!("    [.] Resolved params.outdir to {}", outdir);

                log::info!(
                    "    [.] Resolving params.solc_allowpaths: {:?}",
                    params.solc_allow_paths
                );
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
                log::info!(
                    "    [.] Resolved params.solc_allowpaths to {:?}",
                    solc_allowpaths
                );

                log::info!(
                    "    [.] Resolving params.solc_base_path: {:?}",
                    params.solc_base_path
                );
                let solc_base_path = params.solc_base_path.map(|bp| {
                    PathBuf::from(bp)
                        .canonicalize()
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .to_string()
                });
                log::info!(
                    "    [.] Resolved params.solc_base_path to {:?}",
                    solc_base_path
                );

                log::info!("    [.] Resolving params.solc_remapping");
                let solc_remapping = params.solc_remappings.as_ref().map(|rms| {
                    rms.iter()
                        .map(|rm| repair_remapping(rm.as_str(), None))
                        .collect()
                });
                log::info!(
                    "    [->] Resolved params.solc_remapping:\n    {:#?} to \n    {:#?}",
                    &params.solc_remappings,
                    &solc_remapping
                );

                // Finally, update params with resolved source root and filename.
                // (We don't update earlier to preserve the state of params
                // for error reporting: reporting the parsed in value of
                // `params` will be more helpful to the end user than
                // reporting the modified value of params).
                params.filename = Some(filepath.to_str().unwrap().to_string());
                params.outdir = Some(outdir);
                params.solc_allow_paths = solc_allowpaths;
                params.solc_base_path = solc_base_path;
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
/// and canonicalize
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
    log::debug!(
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
