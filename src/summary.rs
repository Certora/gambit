use std::{collections::HashSet, error, path::PathBuf};

use serde_json::Value;

use crate::SummaryParams;

/// Summarize an existing mutation run (see the [SummaryParams][SummaryParams]
/// struct for detailed documentation)
///
/// [SummaryParams]:crate::cli::SummaryParams
pub fn summarize(params: SummaryParams) -> Result<(), Box<dyn error::Error>> {
    let mutation_dir = PathBuf::from(params.mutation_directory);
    let gambit_results_json_path = mutation_dir.join("gambit_results.json");

    if !&mutation_dir.is_dir() {
        log::error!("Missing mutation directory: `{}`", mutation_dir.display());
        log::error!("Suggestions:");
        log::error!("  [+] Run `gambit mutate` to generate mutants");
        log::error!("  [+] Use the `--mutation-directory` flag to specify a different location");
        std::process::exit(1);
    } else if !&gambit_results_json_path.is_file() {
        log::error!(
            "Missing JSON `{}` in mutation directory `{}`",
            gambit_results_json_path.display(),
            mutation_dir.display()
        );
        log::error!("Suggestions:");
        log::error!("  [+] Run `gambit mutate` to generate mutants");
        log::error!(
            "  [+] Use the `--mutation-directory` flag to specify a different mutation directory"
        );
        std::process::exit(1);
    }
    let gambit_results_json = std::fs::read_to_string(gambit_results_json_path.as_path());
    match gambit_results_json {
        Err(e) => {
            log::error!(
                "Couldn't read results JSON at {}:\nError: {}",
                gambit_results_json_path.display(),
                e
            );
            std::process::exit(1);
        }
        Ok(json) => {
            let v: Value = serde_json::from_str(&json)?;
            if !v.is_array() {
                log::error!(
                    "Ill-formed results JSON found at: {}\nExpected an array",
                    gambit_results_json_path.display()
                );
                std::process::exit(1);
            }
            let v = v.as_array().unwrap();
            match params.mids {
                Some(mids) => {
                    let mids: HashSet<String> = HashSet::from_iter(mids.iter().cloned());
                    for (i, value) in v.iter().enumerate() {
                        let mid = value
                            .as_object()
                            .expect("Expected an object")
                            .get("id")
                            .expect("Expected mutant to have `id` field")
                            .as_i64()
                            .expect("Expected `id` field to be an int")
                            .to_string();
                        if mids.contains(&mid) {
                            print_mutant_summary(i, value);
                        }
                    }
                }
                None => {
                    v.iter().enumerate().for_each(|(i, m)| {
                        print_mutant_summary(i, m);
                    });
                }
            }
        }
    };

    Ok(())
}

/// Print a mutant summary, or a warning if a value is poorly formed.
///
/// # Arguments
///
/// * `i` - the index of the mutated JSON in the `gambit_results.json`. This is
///   used for debug purposes
/// * `mutant_json` - the JSON object from `gambit_results.json` that we are
///   going to summarize. This must have the following keys:
///   - `"id"`: this must map to an integer value
///   - `"diff"`: this must map to a string value
///   - `"name"`: this must map to a string value
///   - `"description"`: this must map to a string value
fn print_mutant_summary(i: usize, mutant_json: &Value) {
    let missing_field_msg = |field_name: &str, i: usize, json: &Value| {
        format!(
            "Missing `\"{}\"` field in entry {} of JSON: {}",
            field_name, i, json
        )
    };
    if let Some(m) = mutant_json.as_object() {
        let mid = m
            .get("id")
            .expect(missing_field_msg("id", i, mutant_json).as_str())
            .as_i64()
            .expect("`id` field should be an int");
        let diff = m
            .get("diff")
            .expect(missing_field_msg("diff", i, mutant_json).as_str())
            .as_str()
            .expect("`diff` field should be a string");
        let name = m
            .get("name")
            .expect(missing_field_msg("name", i, mutant_json).as_str())
            .as_str()
            .expect("`name` field should be a string");
        let desc = m
            .get("description")
            .expect(missing_field_msg("description", i, mutant_json).as_str())
            .as_str()
            .expect("`description` field should be as string");

        println!(
            "\n\n             === {}: {} [{}] ===\n",
            ansi_term::Color::Blue.bold().paint("Mutant ID"),
            ansi_term::Style::new().bold().paint(mid.to_string()),
            ansi_term::Style::new().paint(desc)
        );
        crate::util::print_colorized_unified_diff(diff.to_string());
        println!(
            "\n{}: {}",
            ansi_term::Style::new().bold().paint("Path"),
            name
        );
    } else {
        log::warn!(
            "Expected an object at entry {} but found {}",
            i,
            mutant_json
        );
    }
}
