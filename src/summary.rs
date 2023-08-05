use std::{
    collections::{HashMap, HashSet},
    error,
    path::PathBuf,
};

use serde_json::Value;

use crate::SummaryParams;

struct MutantSummaryEntry {
    mid: String,
    mutant_export_location: String,
    diff: String,
    op_long: String,
    op_short: String,
    orig: String,
    repl: String,
}

/// How should summary operate?
pub enum SummaryMode {
    /// Print high level stats and exit
    PrintStatistics,
    /// Print MIDs specified by `--mids` flag
    PrintSomeMids(Vec<String>),
    /// Print All MIDs
    PrintAllMids,
}

/// Summarize an existing mutation run (see the [SummaryParams][SummaryParams]
/// struct for detailed documentation)
///
/// [SummaryParams]:crate::cli::SummaryParams
pub fn summarize(params: SummaryParams) -> Result<(), Box<dyn error::Error>> {
    let mutation_dir = PathBuf::from(params.mutation_directory);
    let gambit_results_json_path = mutation_dir.join("gambit_results.json");
    let short = params.short;
    let summary_mode = if let Some(mids) = params.mids {
        SummaryMode::PrintSomeMids(mids.clone())
    } else if params.print_all_mids {
        SummaryMode::PrintAllMids
    } else {
        SummaryMode::PrintStatistics
    };

    if !&mutation_dir.is_dir() {
        println!("Missing mutation directory: `{}`", mutation_dir.display());
        println!("Suggestions:");
        println!("  [+] Run `gambit mutate` to generate mutants");
        println!("  [+] Use the `--mutation-directory` flag to specify a different location");
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
            let mutant_summary_entries = v
                .iter()
                .enumerate()
                .map(|(i, m)| get_mutant_summary(i, m))
                .collect::<Vec<Option<MutantSummaryEntry>>>();
            match summary_mode {
                SummaryMode::PrintStatistics => print_statistics(&mutant_summary_entries),
                SummaryMode::PrintSomeMids(mids) => {
                    // Make as a set for fast lookup
                    let mids: HashSet<String> = HashSet::from_iter(mids.iter().cloned());
                    for (_, m) in mutant_summary_entries.iter().enumerate() {
                        if let Some(e) = m {
                            if mids.contains(&e.mid) {
                                print_mutant_summary(&m, short);
                            }
                        }
                    }
                }
                SummaryMode::PrintAllMids => {
                    for m in mutant_summary_entries {
                        print_mutant_summary(&m, short)
                    }
                }
            }
        }
    };

    Ok(())
}

/// Get the `MutantSummary` associated with id `i`
fn get_mutant_summary(i: usize, mutant_json: &Value) -> Option<MutantSummaryEntry> {
    let missing_field_msg = |field_name: &str, i: usize, json: &Value| {
        format!(
            "Missing `\"{}\"` field in entry {} of JSON: {}",
            field_name, i, json
        )
    };
    if let Some(m) = mutant_json.as_object() {
        let mid = m
            .get("id")
            .unwrap_or_else(|| panic!("{}", missing_field_msg("id", i, mutant_json)))
            .as_str()
            .expect("`id` field should be a str");
        let diff = m
            .get("diff")
            .unwrap_or_else(|| panic!("{}", missing_field_msg("diff", i, mutant_json)))
            .as_str()
            .expect("`diff` field should be a string");
        let name = m
            .get("name")
            .unwrap_or_else(|| panic!("{}", missing_field_msg("name", i, mutant_json)))
            .as_str()
            .expect("`name` field should be a string");
        let op_long = m
            .get("description")
            .unwrap_or_else(|| panic!("{}", missing_field_msg("description", i, mutant_json)))
            .as_str()
            .expect("`description` field should be as string");
        let op_short = m
            .get("op")
            .unwrap_or_else(|| panic!("{}", missing_field_msg("op", i, mutant_json)))
            .as_str()
            .expect("`op` field should be as string");
        let orig = m
            .get("orig")
            .unwrap_or_else(|| panic!("{}", missing_field_msg("orig", i, mutant_json)))
            .as_str()
            .expect("`orig` field should be as string");
        let repl = m
            .get("repl")
            .unwrap_or_else(|| panic!("{}", missing_field_msg("repl", i, mutant_json)))
            .as_str()
            .expect("`repl` field should be as string");
        return Some(MutantSummaryEntry {
            mid: mid.to_string(),
            mutant_export_location: name.to_string(),
            diff: diff.to_string(),
            op_long: op_long.to_string(),
            op_short: op_short.to_string(),
            orig: orig.to_string(),
            repl: repl.to_string(),
        });
    } else {
        log::warn!(
            "Expected an object at entry {} but found {}",
            i,
            mutant_json
        );
    }
    return None;
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
fn print_mutant_summary(mutant_summary: &Option<MutantSummaryEntry>, short: bool) {
    if short {
        print_short_mutant_summary(mutant_summary);
    } else {
        print_long_mutant_summary(mutant_summary);
    }
}

fn print_short_mutant_summary(mutant_summary: &Option<MutantSummaryEntry>) {
    if let Some(summary) = mutant_summary {
        println!(
            "({}) {} ({}) {} -> {}",
            ansi_term::Style::new().bold().paint(&summary.mid),
            ansi_term::Color::Blue.bold().paint(&summary.op_short),
            ansi_term::Style::new()
                .italic()
                .paint(&summary.mutant_export_location),
            ansi_term::Color::Green.paint(&summary.orig),
            ansi_term::Color::Red.bold().paint(&summary.repl),
        )
    }
}

fn print_long_mutant_summary(mutant_summary: &Option<MutantSummaryEntry>) {
    if let Some(s) = mutant_summary {
        println!(
            "\n\n             === {}: {} [{}] ===\n",
            ansi_term::Color::Blue.bold().paint("Mutant ID"),
            ansi_term::Style::new().bold().paint(&s.mid),
            ansi_term::Style::new().paint(&s.op_long)
        );
        crate::util::print_colorized_unified_diff(s.diff.clone());
        println!(
            "\n{}: {}",
            ansi_term::Style::new().bold().paint("Path"),
            s.mutant_export_location
        );
    }
}

fn print_statistics(summaries: &Vec<Option<MutantSummaryEntry>>) {
    let mut op_freq: HashMap<String, usize> = HashMap::new();
    let total_mutants = summaries.iter().filter(|s| s.is_some()).count();
    for summary in summaries {
        if let Some(summary) = summary {
            let op = summary.op_short.clone();
            op_freq.insert(op.clone(), op_freq.get(&op).unwrap_or(&0) + 1);
        }
    }
    for (op, freq) in op_freq.iter() {
        println!(
            "{}: {:6} ({:>6.2}%)",
            op,
            freq,
            100.0 * (*freq as f64) / (total_mutants as f64)
        );
    }
    println!("---------------------");
    println!("TOT: {:6} (100.00%)", total_mutants,);
}
