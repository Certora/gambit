use gambit::MutateParams;
use project_root::get_project_root;
use serde_json;
use std::{env, error, path::PathBuf};

fn get_config_json(config_json: &str) -> Result<Vec<MutateParams>, Box<dyn error::Error>> {
    let cwd = env::current_dir()?;
    let project_root = get_project_root()?;
    let path_to_config_json = project_root
        .join("benchmarks")
        .join("config-jsons")
        .join(config_json);
    let p = path_to_config_json.strip_prefix(&cwd).unwrap();
    let json_contents = std::fs::read_to_string(&p)?;
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
    let pb = PathBuf::from(&p);
    let json_parent_directory = pb.parent().unwrap();

    for params in mutate_params.iter_mut() {
        println!("Resolving filename: {}", params.filename.as_ref().unwrap());
        params.filename = params.filename.clone().map(|fnm| {
            json_parent_directory
                .join(fnm)
                .to_str()
                .unwrap()
                .to_string()
        });
        println!("[ -> ] Resolved to: {}", &params.filename.as_ref().unwrap());
    }
    Ok(mutate_params)
}

#[test]
fn test_benchmark_config_json_test1() {
    if let Ok(mutate_params) = get_config_json("test1.json") {
        let results = gambit::run_mutate(mutate_params);
        assert!(results.is_ok());
        let dir_to_mutants = results.unwrap();
        assert!(
            dir_to_mutants.contains_key(&"gambit_out".to_string()),
            "Expected the default output directory `gambit_out`"
        );
        assert_eq!(
            dir_to_mutants.keys().len(),
            1,
            "Expected a single output directory"
        );
        let mutants = dir_to_mutants.get(&"gambit_out".to_string()).unwrap();
        assert_eq!(5, mutants.len(), "Unexpected number of mutants");
    } else {
        assert!(false, "Couldn't read test1.json");
    }
}
