use ansi_term::{Color, Style};
use gambit::MutateParams;
use project_root::get_project_root;
use serde_json;
use std::{collections::HashSet, env, error, path::PathBuf};

#[test]
fn multiple_contracts_1() {
    assert_exact_mutants_from_json(
        "multiple-contracts-1.json",
        &vec![
            //C.get10PoerDecimals
            ("BinaryOpMutation", "**", "+", (22, 24)),
            ("BinaryOpMutation", "**", "-", (22, 24)),
            ("BinaryOpMutation", "**", "*", (22, 24)),
            ("BinaryOpMutation", "**", "/", (22, 24)),
            ("BinaryOpMutation", "**", "%", (22, 24)),
            (
                "SwapArgumentsOperatorMutation",
                "a ** decimals",
                "decimals ** a",
                (22, 23),
            ),
        ],
    )
}

#[test]
fn multiple_contracts_2() {
    assert_exact_mutants_from_json(
        "multiple-contracts-2.json",
        &vec![
            /* Utils.add */
            ("BinaryOpMutation", "+", "-", (9, 17)),
            ("BinaryOpMutation", "+", "*", (9, 17)),
            ("BinaryOpMutation", "+", "/", (9, 17)),
            ("BinaryOpMutation", "+", "%", (9, 17)),
            /* C.get10PowerDecimals */
            ("BinaryOpMutation", "**", "+", (22, 24)),
            ("BinaryOpMutation", "**", "-", (22, 24)),
            ("BinaryOpMutation", "**", "*", (22, 24)),
            ("BinaryOpMutation", "**", "/", (22, 24)),
            ("BinaryOpMutation", "**", "%", (22, 24)),
            (
                "SwapArgumentsOperatorMutation",
                "a ** decimals",
                "decimals ** a",
                (22, 23),
            ),
        ],
    )
}

#[test]
fn multiple_contracts_3() {
    assert_exact_mutants_from_json(
        "multiple-contracts-3.json",
        &vec![
            /* Utils.getarray */
            /* Utils.add */
            ("BinaryOpMutation", "+", "-", (9, 17)),
            ("BinaryOpMutation", "+", "*", (9, 17)),
            ("BinaryOpMutation", "+", "/", (9, 17)),
            ("BinaryOpMutation", "+", "%", (9, 17)),
            /* C.get10PowerDecimals */
            ("BinaryOpMutation", "**", "+", (22, 24)),
            ("BinaryOpMutation", "**", "-", (22, 24)),
            ("BinaryOpMutation", "**", "*", (22, 24)),
            ("BinaryOpMutation", "**", "/", (22, 24)),
            ("BinaryOpMutation", "**", "%", (22, 24)),
            (
                "SwapArgumentsOperatorMutation",
                "a ** decimals",
                "decimals ** a",
                (22, 23),
            ),
            /* C.getarray */
            /* C.callmyself */
            /* C.add */
            ("BinaryOpMutation", "+", "-", (36, 17)),
            ("BinaryOpMutation", "+", "*", (36, 17)),
            ("BinaryOpMutation", "+", "/", (36, 17)),
            ("BinaryOpMutation", "+", "%", (36, 17)),
        ],
    )
}

#[test]
fn multiple_contracts_4() {
    assert_exact_mutants_from_json(
        "multiple-contracts-4.json",
        &vec![
            /* Utils.getarray */
            /* Utils.add */
            ("BinaryOpMutation", "+", "-", (9, 17)),
            ("BinaryOpMutation", "+", "*", (9, 17)),
            ("BinaryOpMutation", "+", "/", (9, 17)),
            ("BinaryOpMutation", "+", "%", (9, 17)),
            /* C.get10PowerDecimals */
            /* C.getarray */
            /* C.callmyself */
            /* C.add */
            ("BinaryOpMutation", "+", "-", (36, 17)),
            ("BinaryOpMutation", "+", "*", (36, 17)),
            ("BinaryOpMutation", "+", "/", (36, 17)),
            ("BinaryOpMutation", "+", "%", (36, 17)),
        ],
    )
}

#[test]
fn test_all() {
    assert_exact_mutants_from_json(
        "all.json",
        &vec![
            /***      DeleteExpressionMutation.sol     ***/
            /* DeleteExpression.myIdentity */
            ("SwapArgumentsOperatorMutation", "i < x", "x < i", (9, 22)),
            ("DeleteExpressionMutation", "i++", "/* i++ */", (9, 29)),
            ("UnaryOperatorMutation", "++", "--", (9, 30)),
            ("UnaryOperatorMutation", "++", "--", (10, 13)),
            /***      ElimDelegateMutation.sol     ***/
            /* B.setVars */
            ("AssignmentMutation", "_num", "0", (10, 15)),
            ("AssignmentMutation", "_num", "1", (10, 15)),
            ("AssignmentMutation", "msg.value", "0", (12, 17)),
            ("AssignmentMutation", "msg.value", "1", (12, 17)),
            /* A.setVars */
            ("ElimDelegateMutation", "delegatecall", "call", (25, 55)),
            ("AssignmentMutation", "success", "true", (28, 23)),
            ("AssignmentMutation", "success", "false", (28, 23)),
        ],
    )
}

/// This performs assertions on a json config. Pass in the config file name
/// (which must be located in the `benchmarks/config-jsons` directory) and a
/// vector of the expected mutants in the form `(orig, replaced, (line:column))`
///
/// The expected mutants can be order independent: we check that the actual and
/// expected mutants have the same number and are the same when put into a set
/// (note: the length check on actuals and expected is maybe a little redundant,
/// but this checks against the same mutant being generated multiple times,
/// which will not show up when the mutants are stored in sets for equality
/// checking)
fn assert_exact_mutants_from_json(json: &str, expected: &Vec<(&str, &str, &str, (usize, usize))>) {
    if let Ok(mutate_params) = get_config_json(json) {
        let results = gambit::run_mutate(mutate_params);
        assert!(results.is_ok());
        let dir_to_mutants = results.unwrap();
        assert_eq!(
            dir_to_mutants.keys().len(),
            1,
            "Expected a single output directory"
        );
        let mutants = dir_to_mutants.values().next().unwrap();
        let actuals: Vec<(String, &str, &str, (usize, usize))> = mutants
            .iter()
            .map(|m| {
                (
                    m.op.to_string(),
                    m.orig.trim(),
                    m.repl.trim(),
                    m.source.get_line_column(m.start).unwrap(),
                )
            })
            .collect();
        let actuals: Vec<(&str, &str, &str, (usize, usize))> = actuals
            .iter()
            .map(|(a, b, c, d)| (a.as_str(), *b, *c, *d))
            .collect();
        assert_eq!(
            expected.len(),
            actuals.len(),
            "\n{} Error: {}: expected {} mutants but found {}.\n    {} {}\n    {}   {}\n",
            Color::Red.bold().paint("[ ! ]"),
            Style::new().bold().underline().italic().paint(json),
            Color::Green.bold().paint(expected.len().to_string()),
            Color::Red.bold().paint(actuals.len().to_string()),
            Color::Green.bold().paint("Expected Mutants:"),
            expected
                .iter()
                .map(|(op, orig, repl, (line, col))| format!(
                    "[{}: `{}` -> `{}` ({}:{})]",
                    op, orig, repl, line, col
                ))
                .collect::<Vec<String>>()
                .join(", "),
            Color::Red.bold().paint("Actual Mutants:"),
            actuals
                .iter()
                .map(|(op, orig, repl, (line, col))| format!(
                    "[{}: `{}` -> `{}` ({}:{})]",
                    op, orig, repl, line, col
                ))
                .collect::<Vec<String>>()
                .join(", ")
        );

        let actuals: HashSet<(&str, &str, &str, (usize, usize))> =
            actuals.iter().cloned().collect();
        let expected: HashSet<(&str, &str, &str, (usize, usize))> =
            expected.iter().cloned().collect();
        assert_eq!(
            expected,
            actuals,
            "\n{} Error: {}\n",
            Color::Red.bold().paint("[ ! ]"),
            Style::new().bold().underline().italic().paint(json),
        );
    } else {
        assert!(false, "Couldn't read test1.json");
    }
}

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
    let base_outdir = PathBuf::from("gambit_tests_out");
    let base_outdir = base_outdir.join(format!("test_{}", config_json));

    for params in mutate_params.iter_mut() {
        // Update outdir
        let outdir = base_outdir.join(&params.outdir);
        params.outdir = outdir.to_str().unwrap().to_string();
        params.filename = params.filename.clone().map(|fnm| {
            json_parent_directory
                .join(fnm)
                .to_str()
                .unwrap()
                .to_string()
        });
        // Update overwrite
        params.overwrite = true;
    }

    Ok(mutate_params)
}
