use std::{io::prelude::*, path::PathBuf};
use tempfile::Builder;

/// Wrap a sequence of statements in a function and write to a temp file,
/// optionally providing a return type
pub fn wrap_and_write_solidity_to_temp_file(
    statements: &[&str],
    params: &[&str],
    returns: Option<&str>,
) -> std::io::Result<PathBuf> {
    // Wrap statements in a Solidity function and contract
    let solidity_code = wrap_solidity(statements, returns, params);
    write_solidity_to_temp_file(solidity_code)
}

/// Write solidity code to a temp file
pub fn write_solidity_to_temp_file(sol: String) -> std::io::Result<PathBuf> {
    // Create a temporary file
    let mut temp_file = Builder::new()
        .prefix("solidity_wrapper_")
        .suffix(".sol")
        .tempfile()?;

    // Write the Solidity code to the temporary file
    writeln!(temp_file, "{}", sol)?;

    // Get the temporary file path
    let temp_path = temp_file.into_temp_path();
    let path = temp_path.to_path_buf();

    // Close and persist the temporary file
    temp_path.persist(&path)?;

    // Return the path of the temporary file
    Ok(path)
}

/// Wrap solidity code in a contract/function
pub fn wrap_solidity(statements: &[&str], returns: Option<&str>, params: &[&str]) -> String {
    let returns = if let Some(returns) = returns {
        format!("({})", returns)
    } else {
        "".into()
    };
    let solidity_code = format!(
        "\
// SPDX-License-Identifier: GPL-3.0-only
pragma solidity ^0.8.0;

contract Wrapper {{
    function wrapped({}) public {} {{
        {}
    }}
}}
    ",
        params.join(", "),
        returns,
        statements.join("\n        ")
    );
    solidity_code
}
