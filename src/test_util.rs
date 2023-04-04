use crate::{SolAST, SolASTVisitor, Solc};
use std::{error, io::prelude::*, path::PathBuf};
use tempfile::Builder;

/// Wrap a sequence of statements in a function and write to a temp file,
/// optionally providing a return type
pub fn wrap_and_write_solidity_to_temp_file(
    statements: &Vec<&str>,
    returns: Option<&str>,
) -> std::io::Result<PathBuf> {
    // Wrap statements in a Solidity function and contract
    let solidity_code = wrap_solidity(statements, returns);

    // Create a temporary file
    let mut temp_file = Builder::new()
        .prefix("solidity_wrapper_")
        .suffix(".sol")
        .tempfile()?;

    // Write the Solidity code to the temporary file
    writeln!(temp_file, "{}", solidity_code)?;

    // Get the temporary file path
    let temp_path = temp_file.into_temp_path();
    let path = temp_path.to_path_buf();

    // Close and persist the temporary file
    temp_path.persist(&path)?;

    // Return the path of the temporary file
    Ok(path)
}

/// Wrap solidity code in a contract/function
pub fn wrap_solidity(statements: &Vec<&str>, returns: Option<&str>) -> String {
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
    function wrapped() public {} {{
        {}
    }}
}}
    ",
        returns,
        statements.join("\n        ")
    );
    solidity_code
}

/// Wrap an expression to a parseable solidity program, write it to disk, and
/// parse it
pub fn parse_expr(expr: &str) -> Result<SolAST, Box<dyn error::Error>> {
    let wrapped = format!("wrapped = {};", expr);
    let solfile = wrap_and_write_solidity_to_temp_file(&vec!["int256 wrapped;", &wrapped], None)?;
    let outdir = Builder::new()
        .prefix("gambit-compile-dir")
        .rand_bytes(5)
        .tempdir()?;
    let solc = Solc::new("solc".into(), PathBuf::from(outdir.into_path()));
    let ast = solc.compile(&solfile)?;
    let v = ExprParserHelper::default();
    let exprs = ast.traverse(&v, ());
    let expr = exprs.get(0).unwrap().clone();
    println!("Expression: {:?}", expr);
    Ok(expr)
}

#[derive(Default)]
struct ExprParserHelper {}

impl SolASTVisitor<(), SolAST> for ExprParserHelper {
    fn visit_node(&self, node: &SolAST, _: &()) -> Option<SolAST> {
        if node.node_type() == Some("Assignment".into()) {
            Some(node.right_hand_side().clone())
        } else {
            None
        }
    }
}

#[cfg(test)]
mod test {
    use std::error;

    use crate::parse_expr;

    #[test]
    pub fn test_parse_expr() -> Result<(), Box<dyn error::Error>> {
        parse_expr("1 + 2")?;
        Ok(())
    }
}
