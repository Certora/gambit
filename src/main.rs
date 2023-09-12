use std::path::{Path, PathBuf};

use clap::Parser;
use gambit::{
    default_gambit_output_directory, normalize_mutation_operator_name, normalize_path,
    print_deprecation_warning, print_error, print_experimental_feature_warning, print_version,
    print_warning, run_mutate, run_summary, Command, MutateParams,
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

fn print_experimental_feature_warnings(params: &MutateParams) {
    // First, check for fallback mutations
    if params.fallback_mutations.is_some() {
        print_experimental_feature_warning("fallback_mutations", "1.0.0");
    }

    let experimental_mutation_operators = vec![("evr", "1.0.0")]
        .iter()
        .map(|item| (normalize_mutation_operator_name(item.0), item.1))
        .collect::<Vec<(String, &str)>>();

    // Collect all mutation operators
    let all_mutations = match (&params.mutations, &params.fallback_mutations) {
        (Some(mutations), None) => mutations.clone(),
        (None, Some(fallback_mutations)) => fallback_mutations.clone(),
        (Some(r1), Some(r2)) => r1.into_iter().chain(r2).cloned().collect(),
        _ => vec![],
    }
    .iter()
    .map(|name| normalize_mutation_operator_name(name))
    .collect::<Vec<String>>();

    for (mutation, version) in experimental_mutation_operators
        .iter()
        .filter(|(experimental_op, _)| all_mutations.contains(experimental_op))
    {
        print_experimental_feature_warning(
            format!("{}:{}", "MutationType", mutation).as_str(),
            version,
        )
    }
}

fn print_deprecation_warnings(params: &MutateParams, is_cli: bool) {
    // We want to format parameter names differently depending on if this
    // parameter was from a CLI invocation or a configuration file
    let format_param = |p: &str| {
        if is_cli {
            format!("--{}", p)
        } else {
            p.to_string()
        }
    };
    if params.sourceroot.is_some() {
        print_deprecation_warning(
            format_param("sourceroot").as_str(),
            "1.0.0",
            "sourceroot is no longer used and will be ignored",
        );
    }

    if params.solc_base_path.is_some() {
        print_deprecation_warning(
            format_param("solc_base_path").as_str(),
            "1.0.0",
            "Use import_path instead",
        );
    }

    if !params.solc_include_paths.is_empty() {
        print_deprecation_warning(
            format_param("solc_include_path").as_str(),
            "1.0.0",
            "Use import_path instead",
        );
    }

    if params.solc_remappings.is_some() {
        print_deprecation_warning(
            format_param("solc_remapping").as_str(),
            "1.0.0",
            "Use import_path instead",
        );
    }
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
    // To tell the difference we deserialize as a `serde_json::Value`
    // and check if it's an array or an object and create a
    // `Vec<MutateParams>` based on this.
    let json_path = &params.json.ok_or("No JSON Path")?;
    log::info!("Running from configuration");
    // Run from config file
    let json_contents = match std::fs::read_to_string(json_path) {
        Ok(contents) => contents,
        Err(e) => {
            gambit::print_file_not_found_error(json_path);
            Err(e)?
        }
    };
    let json: serde_json::Value = serde_json::from_reader(json_contents.as_bytes())?;
    log::info!("Read configuration json: {:#?}", json);

    let mut mutate_params: Vec<MutateParams> = if json.is_array() {
        serde_json::from_str(&json_contents)?
    } else if json.is_object() {
        vec![serde_json::from_str(&json_contents)?]
    } else {
        gambit::print_invalid_conf_error(
            "must be an array or an object",
            "All Gambit configurations must be a valid JSON array or JSON object",
        );
        std::process::exit(1);
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

    // Pass-through args
    let pass_through_outdir = if let Some(outdir) = params.outdir {
        Some(PathBuf::from(".").canonicalize()?.join(outdir))
    } else {
        None
    }
    .map(|pb| pb.to_str().unwrap().to_string());

    for (i, p) in mutate_params.iter_mut().enumerate() {
        // Add pass-through args from CLI to each mutate param we deserialized from the config file

        if params.log_invalid {
            p.log_invalid = true;
        }
        if params.no_export {
            p.no_export = true;
        }
        if params.skip_validate {
            p.skip_validate = true;
        }
        if params.no_overwrite {
            p.no_overwrite = true;
        }
        if params.solc.is_some() {
            p.solc = params.solc.clone();
        }
        if pass_through_outdir.is_some() {
            p.outdir = pass_through_outdir.clone();
        }

        log::info!("Configuration {}", i + 1);

        print_deprecation_warnings(&p, false);
        print_experimental_feature_warnings(&p);

        // PARAM: Filename
        log::info!("    [.] Resolving params.filename");
        let filename = match p.filename {
            Some(ref f) => f.clone(),
            None => {
                gambit::print_invalid_conf_missing_field_error("filename");
                std::process::exit(1);
            }
        };
        let filepath = PathBuf::from(&filename);
        let filepath = if filepath.is_absolute() {
            filepath
        } else {
            let joined = config_parent_pb.join(filepath);
            match joined.canonicalize() {
                Ok(p) => p,
                Err(_) => {
                    gambit::print_file_not_found_error(joined.to_str().unwrap());
                    std::process::exit(1)
                }
            }
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
        let outdir_path = match &p.outdir {
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
            &p.outdir.clone(),
            &outdir,
        );

        // PARAM: solc_allowpaths
        log::debug!("    [.] Resolving params.solc_allow_paths");
        let allow_paths = if let Some(allow_paths) = &p.solc_allow_paths {
            Some(resolve_config_file_paths(
                allow_paths,
                &json_parent_directory,
            )?)
        } else {
            None
        };

        log::debug!(
            "    [.] Resolving params.import_paths: {:?}",
            p.solc_base_path
        );

        let mut import_paths = p
            .import_paths
            .iter()
            .map(
                |ip| match resolve_config_file_path(ip, &json_parent_directory) {
                    Ok(p) => p.to_str().unwrap().to_string(),
                    Err(_) => {
                        gambit::print_file_not_found_error(
                            json_parent_directory.join(ip).to_str().unwrap(),
                        );
                        std::process::exit(1);
                    }
                },
            )
            .collect::<Vec<String>>();

        log::debug!(
            "    [.] Resolving params.solc_base_path: {:?}",
            p.solc_base_path
        );
        if let Some(ref base_path) = p.solc_base_path {
            let base_path = match resolve_config_file_path(base_path, &json_parent_directory) {
                Ok(p) => p.to_str().unwrap().to_string(),
                Err(_) => {
                    gambit::print_file_not_found_error(
                        json_parent_directory.join(base_path).to_str().unwrap(),
                    );
                    std::process::exit(1);
                }
            };

            if !import_paths.contains(&base_path) {
                import_paths.push(base_path);
            }
            p.solc_base_path = None;
        }

        if !p.solc_include_paths.is_empty() {
            for include_path in p.solc_include_paths.iter() {
                let include_path =
                    match resolve_config_file_path(include_path, &json_parent_directory) {
                        Ok(p) => p.to_str().unwrap().to_string(),
                        Err(_) => {
                            gambit::print_file_not_found_error(
                                json_parent_directory.join(include_path).to_str().unwrap(),
                            );
                            std::process::exit(1);
                        }
                    };

                if !import_paths.contains(&include_path) {
                    import_paths.push(include_path);
                }
            }
            p.solc_include_paths = vec![];
        }

        log::debug!("    [->] Resolved params.import_paths: {:?}", import_paths);

        let mut import_maps = vec![];
        for import_map in p.import_maps.iter() {
            import_maps.push(import_map.clone());
        }
        log::debug!("    [.] Resolving params.solc_remapping");
        if let Some(ref remappings) = p.solc_remappings {
            for remapping in remappings.iter() {
                import_maps.push(remapping.clone());
            }
            p.solc_remappings = None;
        }

        if import_paths.is_empty() {
            let default_import_path = json_parent_directory.to_str().unwrap().to_string();
            print_warning(
                "No `import_paths` specified in config",
                format!("Adding default import path {}.\nTo fix, add\n    \"import_paths\": [\"SOME_IMPORT_PATH\"],\nto {}", default_import_path, default_import_path).as_str(),
            );
            import_paths.push(default_import_path);
        }

        log::debug!("    [->] Resolved params.import_maps: {:?}", import_maps);
        p.filename = Some(filepath.to_str().unwrap().to_string());
        p.outdir = Some(outdir);
        p.import_paths = import_paths;
        p.import_maps = import_maps;
        p.solc_allow_paths = allow_paths;
    }
    run_mutate(mutate_params)?;
    Ok(())
}

fn run_mutate_on_filename(mut params: Box<MutateParams>) -> Result<(), Box<dyn std::error::Error>> {
    log::info!("Running CLI MutateParams: {:#?}", &params);

    print_deprecation_warnings(&params, true);
    print_experimental_feature_warnings(&params);

    log::debug!("    [.] Resolving params.filename");
    let filename = match params.filename {
        Some(ref f) => f.clone(),
        None => {
            gambit::print_invalid_conf_missing_field_error("filename");
            std::process::exit(1);
        }
    };
    let filepath = match PathBuf::from(&filename).canonicalize() {
        Ok(path) => path,
        Err(_) => {
            gambit::print_file_not_found_error(&filename);
            std::process::exit(1);
        }
    };

    log::debug!("    [.] Resolving params.outdir {:?}", &params.outdir);

    let outdir = normalize_path(&PathBuf::from(
        &params.outdir.unwrap_or(default_gambit_output_directory()),
    ))
    .to_str()
    .unwrap()
    .to_string();
    log::debug!("    [.] Resolved params.outdir to {}", outdir);

    log::debug!(
        "    [.] Resolving params.solc_allow_paths: {:?}",
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
        "    [.] Resolved params.solc_allow_paths to {:?}",
        solc_allowpaths
    );

    log::debug!(
        "    [.] Resolving params.solc_base_path: {:?}",
        params.solc_base_path
    );

    let mut import_paths = params
        .import_paths
        .iter()
        .filter_map(|ip| {
            if let Ok(path) = PathBuf::from(ip).canonicalize() {
                Some(path.to_str().unwrap().to_string())
            } else {
                print_warning(
                    "Import path not found",
                    format!("Failed to resolve import path {}", ip).as_str(),
                );
                None
            }
        })
        .collect::<Vec<String>>();

    log::debug!("    [->] Resolved params.import_paths: {:?}", import_paths);

    log::debug!(
        "    [.] Resolving params.solc_base_path: {:?}",
        params.solc_base_path
    );
    if let Some(ref base_path) = params.solc_base_path {
        let base_path = match PathBuf::from(&base_path).canonicalize() {
            Ok(p) => p.to_str().unwrap().to_string(),
            Err(_) => {
                gambit::print_file_not_found_error(base_path.as_str());
                std::process::exit(1);
            }
        };
        if !import_paths.contains(&base_path) {
            import_paths.push(base_path);
        }
        params.solc_base_path = None;
    }

    if !params.solc_include_paths.is_empty() {
        for include_path in params.solc_include_paths.iter() {
            let include_path = match PathBuf::from(&include_path).canonicalize() {
                Ok(p) => p.to_str().unwrap().to_string(),
                Err(_) => {
                    gambit::print_file_not_found_error(include_path.as_str());
                    std::process::exit(1);
                }
            };
            if !import_paths.contains(&include_path) {
                import_paths.push(include_path);
            }
        }
        params.solc_include_paths = vec![];
    }
    if import_paths.is_empty() {
        let default_import_path = PathBuf::from(".")
            .canonicalize()
            .ok()
            .map(|x| x.to_str().unwrap().to_string());
        if let Some(path) = default_import_path {
            import_paths.push(path);
        } else {
            print_error(
                "Import Path Error",
                "Could not canonicalize default import path '.'",
            );
            std::process::exit(1);
        }
    }

    log::debug!("    [->] Resolved params.import_paths: {:?}", import_paths);

    let mut import_maps = vec![];
    for import_map in params.import_maps.iter() {
        import_maps.push(import_map.clone());
    }

    if let Some(ref remappings) = params.solc_remappings {
        for remapping in remappings.iter() {
            import_maps.push(remapping.clone());
        }
        params.solc_remappings = None;
    }
    log::debug!("    [->] Resolved params.import_maps: {:?}", import_maps);

    // Finally, update params with resolved source root and filename.
    // (We don't update earlier to preserve the state of params
    // for error reporting: reporting the parsed in value of
    // `params` will be more helpful to the end user than
    // reporting the modified value of params).
    params.filename = Some(filepath.to_str().unwrap().to_string());
    params.outdir = Some(outdir);
    params.solc_allow_paths = solc_allowpaths;
    params.import_paths = import_paths;
    params.solc_remappings = None;

    run_mutate(vec![*params])?;
    Ok(())
}
