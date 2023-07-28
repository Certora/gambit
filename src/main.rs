use std::path::{Path, PathBuf};

use clap::Parser;
use gambit::{
    default_gambit_output_directory, normalize_path, print_deprecation_warning, print_version,
    repair_remapping, run_mutate, run_summary, Command, MutateParams,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = env_logger::builder().try_init();
    match Command::parse() {
        Command::Mutate(params) => {
            if params.json.is_some() {
                run_mutate_on_json(params)?;
            } else {
                run_mutate_on_filename(params)?;
            }
        }
        Command::Summary(params) => {
            run_summary(params)?;
        }
        Command::Version => {
            print_version();
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

fn run_mutate_on_json(params: Box<MutateParams>) -> Result<(), Box<dyn std::error::Error>> {
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
    let json_path = &params.json.ok_or("No JSON Path")?;
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
        let filepath = if filepath.is_absolute() {
            filepath
        } else {
            let joined = config_parent_pb.join(filepath);
            joined
                .canonicalize()
                .expect(format!("Couldn't find file at {}", joined.display()).as_str())
        };

        // PARAM: Outdir
        // + If an absolute `outdir` is specified, normalize it
        // + If a relative (non-absolute) `outdir` is specified,
        //   normalize it with respect to the JSON parent directory
        // + If no `outdir` is specified, use `$CWD/gambit_out`
        //
        // Note: we cannot use `resolve_config_file_path` because
        // `outdir` might not exist yet

        log::debug!("    [.] Resolving params.outdir");
        let outdir_path = match &params.outdir {
            Some(outdir) => {
                let outdir_path = PathBuf::from(outdir);
                if outdir_path.is_absolute() {
                    normalize_path(&outdir_path)
                } else {
                    normalize_path(&json_parent_directory.join(&outdir_path))
                }
            }
            None => normalize_path(&PathBuf::from(".").join(default_gambit_output_directory())),
        };
        let outdir = outdir_path.to_str().unwrap().to_string();
        log::debug!(
            "    [->] Resolved path `{:?}` to `{}`",
            &params.outdir.clone(),
            &outdir,
        );

        // PARAM: solc_allowpaths
        log::debug!("    [.] Resolving params.solc_allow_paths");
        let allow_paths = if let Some(allow_paths) = &params.solc_allow_paths {
            Some(resolve_config_file_paths(
                allow_paths,
                &json_parent_directory,
            )?)
        } else {
            None
        };

        log::debug!(
            "    [.] Resolving params.import_paths: {:?}",
            params.solc_base_path
        );

        let mut import_paths = params
            .import_paths
            .iter()
            .map(|ip| {
                resolve_config_file_path(ip, &json_parent_directory)
                    .expect(
                        format!(
                            "Could not canonicalize path {} with respect to {:?}",
                            ip, &json_parent_directory
                        )
                        .as_str(),
                    )
                    .to_str()
                    .unwrap()
                    .to_string()
            })
            .collect::<Vec<String>>();

        log::debug!(
            "    [.] Resolving params.solc_base_path: {:?}",
            params.solc_base_path
        );
        if let Some(ref base_path) = params.solc_base_path {
            print_deprecation_warning("solc_base_path", "1.0.0", "Use import_path instead");
            let base_path = resolve_config_file_path(&base_path, &json_parent_directory)
                .expect(
                    format!(
                        "Could not canonicalize path {} with respect to {:?}",
                        base_path, &json_parent_directory
                    )
                    .as_str(),
                )
                .to_str()
                .unwrap()
                .to_string();
            if !import_paths.contains(&base_path) {
                import_paths.push(base_path);
            }
            params.solc_base_path = None;
        }

        if !params.solc_include_paths.is_empty() {
            print_deprecation_warning("solc_include_path", "1.0.0", "Use import_path instead");
            for include_path in params.solc_include_paths.iter() {
                let include_path = resolve_config_file_path(include_path, &json_parent_directory)
                    .expect(
                        format!(
                            "Could not canonicalize path {} with respect to {:?}",
                            include_path, &json_parent_directory
                        )
                        .as_str(),
                    )
                    .to_str()
                    .unwrap()
                    .to_string();
                if !import_paths.contains(&include_path) {
                    import_paths.push(include_path);
                }
            }
            params.solc_include_paths = vec![];
        }

        log::debug!("    [->] Resolved params.import_paths: {:?}", import_paths);

        let mut import_maps = vec![];
        for import_map in params.import_maps.iter() {
            let import_map = repair_remapping(
                import_map.as_str(),
                Some(json_parent_directory.to_str().unwrap()),
            );
            import_maps.push(import_map);
        }
        log::debug!("    [.] Resolving params.solc_remapping");
        if let Some(ref remappings) = params.solc_remappings {
            print_deprecation_warning("solc_remapping", "1.0.0", "Use import_map instead");
            for remapping in remappings.iter() {
                let import_map = repair_remapping(
                    remapping.as_str(),
                    Some(json_parent_directory.to_str().unwrap()),
                );
                import_maps.push(import_map);
            }
            params.solc_remappings = None;
        }

        // Finally, update params with resolved source root and filename.
        // (We don't update earlier to preserve the state of params
        // for error reporting: reporting the parsed in value of
        // `params` will be more helpful to the end user than
        // reporting the modified value of params).
        params.filename = Some(filepath.to_str().unwrap().to_string());
        params.outdir = Some(outdir);
        params.import_paths = import_paths;
        params.import_maps = import_maps;
        params.solc_allow_paths = allow_paths;
    }
    run_mutate(mutate_params)?;
    Ok(())
}

fn run_mutate_on_filename(mut params: Box<MutateParams>) -> Result<(), Box<dyn std::error::Error>> {
    log::info!("Running CLI MutateParams: {:#?}", &params);

    // # Path Resolution for CLI Provided Parameters
    log::info!("    Performing File Resolution");
    //                let filename = params.filename.expect("No provided filename");

    log::debug!("    [.] Resolving params.filename");
    let filename = params
        .filename
        .clone()
        .expect("No filename in configuration");
    let filepath = PathBuf::from(&filename).canonicalize().unwrap();

    log::debug!("    [.] Resolving params.outdir {:?}", &params.outdir);

    let outdir = normalize_path(&PathBuf::from(
        &params.outdir.unwrap_or(default_gambit_output_directory()),
    ))
    .to_str()
    .unwrap()
    .to_string();
    log::debug!("    [.] Resolved params.outdir to {}", outdir);

    log::debug!(
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
    log::debug!(
        "    [.] Resolved params.solc_allowpaths to {:?}",
        solc_allowpaths
    );

    log::debug!(
        "    [.] Resolving params.import_paths: {:?}",
        params.solc_base_path
    );

    let mut import_paths = params
        .import_paths
        .iter()
        .map(|ip| {
            PathBuf::from(ip)
                .canonicalize()
                .expect(format!("Could not canonicalize path {}", ip).as_str())
                .to_str()
                .unwrap()
                .to_string()
        })
        .collect::<Vec<String>>();

    log::debug!(
        "    [.] Resolving params.solc_base_path: {:?}",
        params.solc_base_path
    );
    if let Some(ref base_path) = params.solc_base_path {
        print_deprecation_warning("--solc_base_path", "1.0.0", "Use --import_path/-I instead");
        let base_path = PathBuf::from(&base_path)
            .canonicalize()
            .expect(format!("Could not canonicalize path {}", base_path).as_str())
            .to_str()
            .unwrap()
            .to_string();
        if !import_paths.contains(&base_path) {
            import_paths.push(base_path);
        }
        params.solc_base_path = None;
    }

    if !params.solc_include_paths.is_empty() {
        print_deprecation_warning(
            "--solc_include_path",
            "1.0.0",
            "Use --import_path/-I instead",
        );
        for include_path in params.solc_include_paths.iter() {
            let include_path = PathBuf::from(&include_path)
                .canonicalize()
                .expect(format!("Could not canonicalize path {}", include_path).as_str())
                .to_str()
                .unwrap()
                .to_string();
            if !import_paths.contains(&include_path) {
                import_paths.push(include_path);
            }
        }
        params.solc_include_paths = vec![];
    }

    let mut import_maps = vec![];
    for import_map in params.import_maps.iter() {
        let import_map = repair_remapping(import_map.as_str(), None);
        import_maps.push(import_map);
    }
    log::debug!("    [.] Resolving params.solc_remapping");
    if let Some(ref remappings) = params.solc_remappings {
        print_deprecation_warning("--solc_remapping", "1.0.0", "Use --import_map/-m instead");
        for remapping in remappings.iter() {
            let import_map = repair_remapping(remapping.as_str(), None);
            import_maps.push(import_map);
        }
        params.solc_remappings = None;
    }
    log::debug!("    [.] Resolving params.solc_remapping");
    let solc_remapping = params.solc_remappings.as_ref().map(|rms| {
        rms.iter()
            .map(|rm| repair_remapping(rm.as_str(), None))
            .collect()
    });
    log::debug!(
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
    params.import_paths = import_paths;
    params.solc_remappings = solc_remapping;

    run_mutate(vec![*params])?;
    Ok(())
}
