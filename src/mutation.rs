use crate::{get_indent, Source};
use clap::ValueEnum;
use num_bigint::BigInt;
use num_traits::{One, Zero};
use serde::{Deserialize, Serialize};
use solang::sema::ast::{Expression, RetrieveType, Statement};
use solang_parser::pt::CodeLocation;
use std::{error, fmt::Display, rc::Rc};

/// This struct describes a mutant.
#[derive(Debug, Clone)]
pub struct Mutant {
    /// The original program's source
    pub source: Rc<Source>,

    /// The mutation operator that was applied to generate this mutant
    pub op: MutationType,

    /// The string representation of the original node
    pub orig: String,

    /// The index into the program source marking the beginning (inclusive) of
    /// the source to be replaced
    pub start: usize,

    /// The index into the program source marking the end (inclusive) of the
    /// source to be replaced
    pub end: usize,

    /// The string replacement
    pub repl: String,
}

impl Mutant {
    pub fn new(
        source: Rc<Source>,
        op: MutationType,
        start: usize,
        end: usize,
        repl: String,
    ) -> Mutant {
        let orig = String::from_utf8(source.contents()[start..end].to_vec()).unwrap();
        Mutant {
            source,
            op,
            orig,
            start,
            end,
            repl,
        }
    }

    /// Render this mutant as String with the full source file contents
    ///
    /// TODO: Cache these contents: this data might be needed multiple times,
    /// and if so this should be cached as it currently involves file IO (though
    /// Source::contents() should also be cached)
    pub fn as_source_string(&self) -> Result<String, Box<dyn error::Error>> {
        let contents = self.source.contents();
        let prelude = &contents[0..self.start];
        let postlude = &contents[self.end..contents.len()];

        let res = [prelude, self.repl.as_bytes(), postlude].concat();
        let mut_string = String::from_utf8(res)?;
        let mut lines = mut_string.lines();

        let (line, _) = self.source.get_line_column(self.start)?;
        let mut lines2 = vec![];
        for _ in 1..line {
            lines2.push(lines.next().unwrap());
        }

        let mut_line = lines.next().unwrap();
        let orig_string = String::from_utf8(contents.to_vec())?;
        let orig_line = orig_string.lines().nth(line - 1).unwrap();

        let indent = get_indent(mut_line);
        let comment = format!(
            "{}/// {}(`{}` |==> `{}`) of: `{}`",
            indent,
            self.op.to_string(),
            self.orig.trim(),
            self.repl,
            orig_line.trim()
        );
        lines2.push(&comment);
        lines2.push(mut_line);

        for line in lines {
            lines2.push(line);
        }

        // XXX: this is a hack to avoid trailing newline diffs
        if contents.last().unwrap() == &b'\n' {
            lines2.push("");
        }
        Ok(lines2.join("\n"))
    }

    pub fn get_line_column(&self) -> Result<(usize, usize), Box<dyn error::Error>> {
        self.source.get_line_column(self.start)
    }
}

impl Display for Mutant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let contents = self.source.contents();
        let (start, end) = (self.start, self.end);
        let orig = &contents[start..end];
        let repl = &self.repl;
        write!(
            f,
            "{}: {} |==> {}",
            self.op.to_string(),
            String::from_utf8_lossy(orig),
            repl
        )
    }
}

/// Every kind of mutation implements this trait. A mutation can check if it
/// applies to an AST node, and can mutate an AST node.
pub trait Mutation {
    fn mutate_statement(&self, _stmt: &Statement, source: &Rc<Source>) -> Vec<Mutant>;

    fn mutate_expression(&self, _expr: &Expression, source: &Rc<Source>) -> Vec<Mutant>;
}

/// Kinds of mutations.
#[derive(Hash, Eq, PartialEq, Clone, Copy, Debug, ValueEnum, Deserialize, Serialize)]
pub enum MutationType {
    // # New Operators
    // ## Literal Value Replacement
    LiteralValueReplacement,
    // ## Binary Operator Replacement
    BitwiseOperatorReplacement,
    RelationalOperatorReplacement,
    ArithmeticOperatorReplacement,
    LogicalOperatorReplacement,
    ShiftOperatorReplacement,
    // ## UnaryOperatorReplacement
    UnaryOperatorReplacement,
    // ## Fallback Operators
    ExpressionValueReplacement,
    StatementDeletion,

    // # Old Operators (Deprecated)
    AssignmentMutation,
    BinaryOpMutation,
    DeleteExpressionMutation,
    ElimDelegateMutation,
    FunctionCallMutation,
    // IfStatementMutation,
    RequireMutation,
    SwapArgumentsFunctionMutation,
    SwapArgumentsOperatorMutation,
    UnaryOperatorMutation,
}

impl ToString for MutationType {
    fn to_string(&self) -> String {
        let str = match self {
            MutationType::LiteralValueReplacement => "LiteralValueReplacement",
            MutationType::BitwiseOperatorReplacement => "ConditionalOperatorReplacement",
            MutationType::RelationalOperatorReplacement => "RelationalOperatorReplacement",
            MutationType::ArithmeticOperatorReplacement => "ArithmeticOperatorReplacemnt",
            MutationType::LogicalOperatorReplacement => "LogicalOperatorReplacement",
            MutationType::ShiftOperatorReplacement => "ShiftOperatorReplacement",
            MutationType::UnaryOperatorReplacement => "UnaryOperatorReplacement",
            MutationType::ExpressionValueReplacement => "ExpressionOperatorReplacement",
            MutationType::StatementDeletion => "StatementDeletion",

            MutationType::AssignmentMutation => "AssignmentMutation",
            MutationType::BinaryOpMutation => "BinaryOpMutation",
            MutationType::DeleteExpressionMutation => "DeleteExpressionMutation",
            MutationType::ElimDelegateMutation => "ElimDelegateMutation",
            MutationType::FunctionCallMutation => "FunctionCallMutation",
            // MutationType::IfStatementMutation => "IfStatementMutation",
            MutationType::RequireMutation => "RequireMutation",
            MutationType::SwapArgumentsFunctionMutation => "SwapArgumentsFunctionMutation",
            MutationType::SwapArgumentsOperatorMutation => "SwapArgumentsOperatorMutation",
            MutationType::UnaryOperatorMutation => "UnaryOperatorMutation",
        };
        str.into()
    }
}

impl Mutation for MutationType {
    fn mutate_statement(&self, stmt: &Statement, source: &Rc<Source>) -> Vec<Mutant> {
        let loc = stmt.loc();
        if let None = loc.try_file_no() {
            return vec![];
        }
        match self {
            MutationType::StatementDeletion => vec![Mutant::new(
                source.clone(),
                self.clone(),
                stmt.loc().start(),
                stmt.loc().end() + 1,
                "".to_string(),
            )],
            _ => vec![],
        }
    }

    fn mutate_expression(&self, expr: &Expression, source: &Rc<Source>) -> Vec<Mutant> {
        match self {
            // Binary Operators
            MutationType::ArithmeticOperatorReplacement => arith_op_replacement(self, expr, source),
            MutationType::ShiftOperatorReplacement => shift_op_replacement(self, expr, source),
            MutationType::BitwiseOperatorReplacement => bitwise_op_replacement(self, expr, source),
            MutationType::RelationalOperatorReplacement => rel_op_replacement(self, expr, source),
            MutationType::LogicalOperatorReplacement => logical_op_replacement(self, expr, source),
            // Other
            MutationType::LiteralValueReplacement => literal_value_replacement(self, expr, source),
            MutationType::UnaryOperatorReplacement => todo!(),
            MutationType::ExpressionValueReplacement => todo!(),

            // Old Operators
            MutationType::AssignmentMutation => todo!(),
            MutationType::BinaryOpMutation => todo!(),
            MutationType::DeleteExpressionMutation => todo!(),
            MutationType::ElimDelegateMutation => todo!(),
            MutationType::FunctionCallMutation => todo!(),
            MutationType::RequireMutation => todo!(),
            MutationType::SwapArgumentsFunctionMutation => todo!(),
            MutationType::SwapArgumentsOperatorMutation => todo!(),
            MutationType::UnaryOperatorMutation => todo!(),
            _ => vec![],
        }
    }
}

impl MutationType {
    pub fn default_mutation_operators() -> Vec<MutationType> {
        vec![
            MutationType::AssignmentMutation,
            MutationType::BinaryOpMutation,
            MutationType::DeleteExpressionMutation,
            MutationType::ElimDelegateMutation,
            MutationType::FunctionCallMutation,
            // MutationType::IfStatementMutation,
            MutationType::RequireMutation,
            // MutationType::SwapArgumentsFunctionMutation,
            MutationType::SwapArgumentsOperatorMutation,
            MutationType::UnaryOperatorMutation,
        ]
    }
}

/// Get a string representation of an operator
fn get_operator(expr: &Expression) -> &str {
    match expr {
        Expression::Add { .. } => "+",
        Expression::Subtract { .. } => "-",
        Expression::Multiply { .. } => "*",
        Expression::Divide { .. } => "/",
        Expression::Modulo { .. } => "%",
        Expression::Power { .. } => "**",
        Expression::BitwiseOr { .. } => "|",
        Expression::BitwiseAnd { .. } => "&",
        Expression::BitwiseXor { .. } => "^",
        Expression::ShiftLeft { .. } => "<<",
        Expression::ShiftRight { .. } => ">>",
        Expression::PreIncrement { .. } => "++",
        Expression::PreDecrement { .. } => "--",
        Expression::PostIncrement { .. } => "++",
        Expression::PostDecrement { .. } => "--",
        Expression::More { .. } => ">",
        Expression::Less { .. } => "<",
        Expression::MoreEqual { .. } => ">=",
        Expression::LessEqual { .. } => "<=",
        Expression::Equal { .. } => "==",
        Expression::NotEqual { .. } => "!=",
        Expression::Not { .. } => "!",
        Expression::BitwiseNot { .. } => "~",
        Expression::Negate { .. } => "-",
        Expression::ConditionalOperator { .. } => "?",
        Expression::Or { .. } => "||",
        Expression::And { .. } => "&&",
        _ => "",
    }
}

fn arith_op_replacement(op: &MutationType, expr: &Expression, source: &Rc<Source>) -> Vec<Mutant> {
    let loc = expr.loc();
    let arith_op = get_operator(expr);
    let rs = vec!["+", "-", "*", "/", "**", "%"];
    let replacements: Vec<&&str> = rs.iter().filter(|x| **x != arith_op).collect();

    if let None = loc.try_file_no() {
        return vec![];
    }
    match expr {
        Expression::BitwiseOr { left, right, .. }
        | Expression::BitwiseAnd { left, right, .. }
        | Expression::BitwiseXor { left, right, .. }
        | Expression::Divide { left, right, .. }
        | Expression::Modulo { left, right, .. }
        | Expression::Multiply { left, right, .. }
        | Expression::Subtract { left, right, .. }
        | Expression::Add { left, right, .. } => {
            let (start, end) = (left.loc().end(), right.loc().start());
            replacements
                .iter()
                .map(|r| Mutant::new(source.clone(), op.clone(), start, end, format!(" {} ", r)))
                .collect()
        }
        Expression::Power { base, exp, .. } => {
            let (start, end) = (base.loc().end(), exp.loc().start());
            replacements
                .iter()
                .map(|r| Mutant::new(source.clone(), op.clone(), start, end, format!(" {} ", r)))
                .collect()
        }
        _ => vec![],
    }
}

fn bitwise_op_replacement(
    op: &MutationType,
    expr: &Expression,
    source: &Rc<Source>,
) -> Vec<Mutant> {
    let loc = expr.loc();
    let bitwise_op = get_operator(expr);
    let rs = vec!["|", "&", "^"];
    let replacements: Vec<&&str> = rs.iter().filter(|x| **x != bitwise_op).collect();

    if let None = loc.try_file_no() {
        return vec![];
    }
    match expr {
        Expression::BitwiseOr { left, right, .. }
        | Expression::BitwiseAnd { left, right, .. }
        | Expression::BitwiseXor { left, right, .. } => {
            let (start, end) = (left.loc().end(), right.loc().start());
            replacements
                .iter()
                .map(|r| Mutant::new(source.clone(), op.clone(), start, end, format!(" {} ", r)))
                .collect()
        }
        _ => vec![],
    }
}

fn shift_op_replacement(op: &MutationType, expr: &Expression, source: &Rc<Source>) -> Vec<Mutant> {
    let loc = expr.loc();
    let shift_op = get_operator(expr);
    let rs = vec!["<<", ">>"];
    let replacements: Vec<&&str> = rs.iter().filter(|x| **x != shift_op).collect();

    if let None = loc.try_file_no() {
        return vec![];
    }
    match expr {
        Expression::ShiftLeft { left, right, .. } | Expression::ShiftRight { left, right, .. } => {
            let (start, end) = (left.loc().end(), right.loc().start());
            replacements
                .iter()
                .map(|r| Mutant::new(source.clone(), op.clone(), start, end, format!(" {} ", r)))
                .collect()
        }
        _ => vec![],
    }
}

fn rel_op_replacement(op: &MutationType, expr: &Expression, source: &Rc<Source>) -> Vec<Mutant> {
    let loc = expr.loc();
    if let None = loc.try_file_no() {
        return vec![];
    }

    // We need to know two things to perform a mutation:
    // 1. The replacement string
    // 2. The start, stop of each replacement
    //
    // For true and false replacements we are replacing the full expression, and
    // we can get bounds from `expr`. For relational replacements we need to
    // know the bounds of the binary operator, which we get from left and right.
    //
    // Thus, replacements is a tuple of (replacements, (start, end)), where
    // (start, end) are the binary operator's start and end locations (note,
    // these are only used for replacing with another operator, otherwise the
    // `expr.loc` values are used)
    let (replacements, bounds) = match expr {
        Expression::Less { left, right, .. } => (
            vec!["<=", "!=", "false"],
            (left.loc().end(), right.loc().start()),
        ),
        Expression::LessEqual { left, right, .. } => (
            vec!["<", "==", "true"],
            (left.loc().end(), right.loc().start()),
        ),
        Expression::More { left, right, .. } => (
            vec![">=", "!=", "false"],
            (left.loc().end(), right.loc().start()),
        ),
        Expression::MoreEqual { left, right, .. } => (
            vec![">", "==", "true"],
            (left.loc().end(), right.loc().start()),
        ),
        Expression::Equal { left, right, .. } => {
            // Assuming that we only need the left type to determine legal mutations
            match left.ty() {
                // The following types are orderable, so we use those for better mutation operators
                solang::sema::ast::Type::Int(_)
                | solang::sema::ast::Type::Uint(_)
                | solang::sema::ast::Type::Rational => (
                    vec!["<=", ">=", "false"],
                    (left.loc().end(), right.loc().start()),
                ),

                // The following types are not orderable, so we replace with true and false
                // TODO: Can Addresses be ordered?
                solang::sema::ast::Type::Address(_) => (vec!["true", "false"], (0, 0)),
                _ => (vec!["true", "false"], (0, 0)),
            }
        }
        Expression::NotEqual { left, right, .. } => {
            // Assuming that we only need the left type to determine legal mutations
            match left.ty() {
                // The following types are orderable, so we use those for better mutation operators
                solang::sema::ast::Type::Int(_)
                | solang::sema::ast::Type::Uint(_)
                | solang::sema::ast::Type::Rational => (
                    vec!["< ", "> ", "true"],
                    (left.loc().end(), right.loc().start()),
                ),

                // The following types are not orderable, so we replace with true and false
                // TODO: Can Addresses be ordered?
                solang::sema::ast::Type::Address(_) => (vec!["true", "false"], (0, 0)),
                _ => (vec!["true", "false"], (0, 0)),
            }
        }
        _ => (vec![], (0, 0)),
    };

    // Now, apply the replacements
    let mut mutants = vec![];
    let expr_start = expr.loc().start();
    let expr_end = expr.loc().end();
    let op_start = bounds.0;
    let op_end = bounds.1;
    for r in replacements {
        mutants.push(match r {
            "true" | "false" => Mutant::new(
                source.clone(),
                op.clone(),
                expr_start,
                expr_end,
                r.to_string(),
            ),
            _ => Mutant::new(source.clone(), op.clone(), op_start, op_end, r.to_string()),
        });
    }
    mutants
}

fn logical_op_replacement(
    op: &MutationType,
    expr: &Expression,
    source: &Rc<Source>,
) -> Vec<Mutant> {
    let loc = expr.loc();
    if let None = loc.try_file_no() {
        return vec![];
    }

    let replacements = match expr {
        Expression::And { left, right, .. } => vec![
            ("LHS", left.loc().start(), left.loc().end()),
            ("RHS", right.loc().start(), right.loc().end()),
            ("false", 0, 0),
        ],
        Expression::Or { left, right, .. } => vec![
            ("LHS", left.loc().start(), left.loc().end()),
            ("RHS", right.loc().start(), right.loc().end()),
            ("true", 0, 0),
        ],
        _ => vec![],
    };

    // Now, apply the replacements
    let mut mutants = vec![];
    let expr_start = expr.loc().start();
    let expr_end = expr.loc().end();
    for (r, s, e) in replacements {
        mutants.push(match r {
            "LHS" | "RHS" => {
                let repl = std::str::from_utf8(source.contents_between_offsets(s, e))
                    .unwrap()
                    .to_string();
                Mutant::new(source.clone(), op.clone(), expr_start, expr_end, repl)
            }
            "true" | "false" => Mutant::new(
                source.clone(),
                op.clone(),
                expr_start,
                expr_end,
                r.to_string(),
            ),
            _ => panic!("Illegal State"),
        });
    }
    mutants
}

fn literal_value_replacement(
    op: &MutationType,
    expr: &Expression,
    source: &Rc<Source>,
) -> Vec<Mutant> {
    let loc = expr.loc();
    if let None = loc.try_file_no() {
        return vec![];
    }
    // We are only replacing BoolLiterals, NumberLiterals, and
    // RationalNumberLiterals. It's not clear what other literals we should
    // replace
    let replacements = match expr {
        Expression::BoolLiteral { value, .. } => vec![(!value).to_string()],
        Expression::NumberLiteral { ty, value, .. } => match ty {
            solang::sema::ast::Type::Address(_) => todo!(),
            solang::sema::ast::Type::Int(_) => {
                if value.is_zero() {
                    vec!["-1".to_string(), "1".to_string()]
                } else {
                    vec!["0".to_string(), (-value).to_string()]
                }
            }
            solang::sema::ast::Type::Uint(_) => {
                if value.is_zero() {
                    vec!["1".to_string()]
                } else {
                    vec!["0".to_string(), (value + BigInt::one()).to_string()]
                }
            }
            _ => vec![],
        },
        Expression::RationalNumberLiteral { value: _, .. } => vec![],
        Expression::BytesLiteral { .. } => vec![],
        Expression::CodeLiteral { .. } => vec![],
        Expression::StructLiteral { .. } => vec![],
        Expression::ArrayLiteral { .. } => vec![],
        Expression::ConstArrayLiteral { .. } => vec![],
        _ => vec![],
    };
    let mut mutants = vec![];
    let expr_start = expr.loc().start();
    let expr_end = expr.loc().end();
    for r in replacements {
        mutants.push(Mutant::new(
            source.clone(),
            op.clone(),
            expr_start,
            expr_end,
            r.clone(),
        ));
    }
    mutants
}

/// This testing module defines and uses the testing infrastructure, allowing
/// for varying degrees of testing flexibility.
///
/// First, we define two types of assert functions:
///
/// 1. `assert_num_mutants_for_XXX`: make an assertion for the number of mutants
///    generated by a provided set of operators for the given code
///
/// 2. `assert_exact_mutants_for_XXX`: make an assertion on the exact mutants
///    generated by a provided set of operators for the given code
///
/// Next, we define two forms of code to make assertions about:
///
/// 1. **Statements:** we represent a list of statements as a `Vec<&str>`, and
///    this makes it easy to write a simple program to mutate, e.g.,
///
///    ```
///    vec!["uint256 a = 1;", "uint256 b = 2;", "uint256 c = a + b;"]
///    ```
///
///    Functions `assert_exact_mutants_for_statements()` and
///    `assert_num_mutants_for_statements()` use this program form to make
///    assertions about mutations
///
/// 2. **Full Source:** sometimes we want to be able to specify a full source file.
///    This is more verbose than using statements (as we need to write more
///    boilerplate), but we have maximal flexibility in the code we are mutating.
///
///    Funcctions `assert_exact_mutants_for_source()` and
///    `assert_num_mutants_for_source()` use this program form to make
///    assertions about mutations
#[cfg(test)]
mod test {
    use crate::test_util::*;
    use crate::{MutationType, MutationType::*, Mutator, MutatorConf, Solc, Source};
    use std::collections::HashSet;
    use std::path::PathBuf;
    use std::rc::Rc;
    use std::{error, path::Path};
    use tempfile::Builder;

    #[test]
    pub fn test_assignment_mutation() -> Result<(), Box<dyn error::Error>> {
        let ops = vec![AssignmentMutation];
        assert_exact_mutants_for_statements(
            &vec!["uint256 x;", "x = 3;"],
            &ops,
            &vec!["(-1)", "0", "1"],
        );
        assert_exact_mutants_for_statements(&vec!["int256 x;", "x = 1;"], &ops, &vec!["(-1)", "0"]);
        assert_exact_mutants_for_statements(&vec!["int256 x;", "x = 0;"], &ops, &vec!["(-1)", "1"]);
        // FIXME: The following three test cases are BROKEN!! Currently these
        // all get mutated to [-1, 1, 0, false, true] because they are not
        // 'number's. Validation would strip out the true/false. We would want
        // constant propagation to strip out the 0 in `x = -0`
        assert_num_mutants_for_statements(
            &vec!["int256 x;", "x = -2;"],
            &vec![AssignmentMutation],
            5,
        );
        assert_num_mutants_for_statements(
            &vec!["int256 x;", "x = -1;"],
            &vec![AssignmentMutation],
            5,
        );
        assert_num_mutants_for_statements(
            &vec!["int256 x;", "x = -0;"],
            &vec![AssignmentMutation],
            5,
        );

        assert_exact_mutants_for_statements(&vec!["bool b;", "b = true;"], &ops, &vec!["false"]);
        assert_exact_mutants_for_statements(&vec!["bool b;", "b = false;"], &ops, &vec!["true"]);

        Ok(())
    }

    #[test]
    pub fn test_binary_op_mutation() -> Result<(), Box<dyn error::Error>> {
        let ops = vec![BinaryOpMutation];
        let repls = vec!["+", "-", "*", "/", "%", "**"];
        // Closure to drop the given operator for he set of replacements
        let without = |s: &str| {
            let r: Vec<&str> = repls
                .iter()
                .filter(|r| !s.eq(**r))
                .map(|s| s.clone())
                .collect();
            r
        };
        assert_exact_mutants_for_statements(&vec!["uint256 x = 1 + 2;"], &ops, &without("+"));
        assert_exact_mutants_for_statements(&vec!["uint256 x = 2 - 1;"], &ops, &without("-"));
        assert_exact_mutants_for_statements(&vec!["uint256 x = 1 * 2;"], &ops, &without("*"));
        assert_exact_mutants_for_statements(&vec!["uint256 x = 2 / 1;"], &ops, &without("/"));
        assert_exact_mutants_for_statements(&vec!["uint256 x = 1 % 2;"], &ops, &without("%"));
        assert_exact_mutants_for_statements(&vec!["uint256 x = 1 ** 2;"], &ops, &without("**"));
        Ok(())
    }

    #[test]
    pub fn test_delete_expression_mutation() -> Result<(), Box<dyn error::Error>> {
        let ops = vec![DeleteExpressionMutation];
        assert_exact_mutants_for_statements(&vec!["gasleft();"], &ops, &vec!["/* gasleft() */"]);
        assert_exact_mutants_for_statements(
            &vec!["uint256 x = 0;", "x = 3;"],
            &ops,
            &vec!["/* x = 3 */"],
        );
        Ok(())
    }

    #[test]
    pub fn test_elim_delegate_mutation() -> Result<(), Box<dyn error::Error>> {
        let _ops = vec![ElimDelegateMutation];
        // TODO: how should I test this?
        let code = "\
// SPDX-License-Identifier: GPL-3.0-only
pragma solidity ^0.8.9;

contract B {
    uint public num;
    address public sender;
    uint public value;

    function setVars(uint _num) public payable {
        num = _num;
        sender = msg.sender;
        value = msg.value;
    }
}

contract A {
    uint public num;
    address public sender;
    uint public value;
    bool public delegateSuccessful;
    bytes public myData;
    

    function setVars(address _contract, uint _num) public payable {
        (bool success, bytes memory data) = _contract.delegatecall(
            abi.encodeWithSignature(\"setVars(uint256)\", _num)
        );
	delegateSuccessful = success;
	myData = data;
    }
}
";
        let ops = vec![MutationType::ElimDelegateMutation];
        let expected = vec!["call"];
        assert_exact_mutants_for_source(code, &ops, &expected);
        Ok(())
    }

    #[test]
    pub fn test_function_call_mutation() -> Result<(), Box<dyn error::Error>> {
        let _ops = vec![FunctionCallMutation];
        // TODO: how should I test this?
        Ok(())
    }

    // #[test]
    // pub fn test_if_statement_mutation() -> Result<(), Box<dyn error::Error>> {
    //     let ops = vec![IfStatementMutation];
    //     assert_num_mutants_for_statements(
    //         &vec!["uint256 x;", "if (true) { x = 1; } else { x = 2 ;}"],
    //         &ops,
    //         1,
    //     );
    //     assert_num_mutants_for_statements(&vec!["if (true) {}"], &ops, 1);
    //     Ok(())
    // }

    #[test]
    pub fn test_require_mutation() -> Result<(), Box<dyn error::Error>> {
        let ops = vec![RequireMutation];
        assert_num_mutants_for_statements(&vec!["bool c = true;", "require(c);"], &ops, 2);
        assert_num_mutants_for_statements(&vec!["require(true);"], &ops, 1);
        assert_num_mutants_for_statements(
            &vec!["bool a = true;", "bool b = false;", "require(a && b);"],
            &ops,
            2,
        );
        Ok(())
    }

    #[test]
    pub fn test_unary_op_mutation() -> Result<(), Box<dyn error::Error>> {
        let ops = vec![UnaryOperatorMutation];
        let prefix = vec!["++", "--", "~"];
        let suffix = vec!["++", "--"];

        // Closure to drop the given operator for he set of replacements
        let without_prefix = |s: &str| {
            let r: Vec<&str> = prefix
                .iter()
                .filter(|r| !s.eq(**r))
                .map(|s| s.clone())
                .collect();
            r
        };
        let without_suffix = |s: &str| {
            let r: Vec<&str> = suffix
                .iter()
                .filter(|r| !s.eq(**r))
                .map(|s| s.clone())
                .collect();
            r
        };
        assert_exact_mutants_for_statements(
            &vec!["uint256 a = 10;", "uint256 x = ++a;"],
            &ops,
            &without_prefix("++"),
        );
        assert_exact_mutants_for_statements(
            &vec!["uint256 a = 10;", "uint256 x = --a;"],
            &ops,
            &without_prefix("--"),
        );
        assert_exact_mutants_for_statements(
            &vec!["uint256 a = 10;", "uint256 x = ~a;"],
            &ops,
            &without_prefix("~"),
        );
        assert_exact_mutants_for_statements(
            &vec!["uint256 a = 10;", "uint256 x = a--;"],
            &ops,
            &without_suffix("--"),
        );
        assert_exact_mutants_for_statements(
            &vec!["uint256 a = 10;", "uint256 x = a++;"],
            &ops,
            &without_suffix("++"),
        );
        Ok(())
    }

    fn assert_num_mutants_for_statements(
        statements: &Vec<&str>,
        ops: &Vec<MutationType>,
        expected: usize,
    ) {
        let mutator = apply_mutation_to_statements(statements, None, ops).unwrap();
        assert_eq!(
            expected,
            mutator.mutants().len(),
            "Error: applied ops\n   -> {:?}\nto program\n  -> {:?}\nat {:?} for more info",
            ops,
            statements.join("   "),
            mutator
                .sources
                .iter()
                .map(|s| s.filename())
                .collect::<Vec<&Path>>()
        );
    }

    fn assert_exact_mutants_for_statements(
        statements: &Vec<&str>,
        ops: &Vec<MutationType>,
        expected: &Vec<&str>,
    ) {
        let mutator = apply_mutation_to_statements(statements, None, ops).unwrap();
        assert_eq!(
            expected.len(),
            mutator.mutants().len(),
            "Error: applied ops\n   -> {:?}\nto program\n  -> {:?}\nat {:?} for more info",
            ops,
            statements.join("   "),
            mutator
                .sources
                .iter()
                .map(|s| s.filename())
                .collect::<Vec<&Path>>()
        );

        let actuals: HashSet<&str> = mutator.mutants().iter().map(|m| m.repl.as_str()).collect();
        let expected: HashSet<&str> = expected.iter().map(|s| *s).collect();
        assert_eq!(actuals, expected);
    }

    fn apply_mutation_to_statements(
        statements: &Vec<&str>,
        returns: Option<&str>,
        ops: &Vec<MutationType>,
    ) -> Result<Mutator, Box<dyn error::Error>> {
        let source = wrap_and_write_solidity_to_temp_file(statements, returns).unwrap();
        let outdir = Builder::new()
            .prefix("gambit-compile-dir")
            .rand_bytes(5)
            .tempdir()?;
        let mut mutator = make_mutator(ops, source, outdir.into_path());
        let sources = mutator.sources().clone();
        mutator.mutate(sources)?;

        Ok(mutator)
    }

    fn _assert_num_mutants_for_source(source: &str, ops: &Vec<MutationType>, expected: usize) {
        let mutator = apply_mutation_to_source(source, ops).unwrap();
        assert_eq!(
            expected,
            mutator.mutants().len(),
            "Error: applied ops\n   -> {:?}\nto program\n  -> {:?}\n\nSee {:?} for more info",
            ops,
            source,
            mutator
                .sources
                .iter()
                .map(|s| s.filename())
                .collect::<Vec<&Path>>()
        );
    }

    fn assert_exact_mutants_for_source(
        source: &str,
        ops: &Vec<MutationType>,
        expected: &Vec<&str>,
    ) {
        let mutator = apply_mutation_to_source(source, ops).unwrap();
        assert_eq!(
            expected.len(),
            mutator.mutants().len(),
            "Error: applied ops\n   -> {:?}\nto program\n  -> {:?}\nat {:?} for more info",
            ops,
            source,
            mutator
                .sources
                .iter()
                .map(|s| s.filename())
                .collect::<Vec<&Path>>()
        );

        let actuals: HashSet<&str> = mutator.mutants().iter().map(|m| m.repl.as_str()).collect();
        let expected: HashSet<&str> = expected.iter().map(|s| *s).collect();
        assert_eq!(actuals, expected);
    }

    fn apply_mutation_to_source(
        source: &str,
        ops: &Vec<MutationType>,
    ) -> Result<Mutator, Box<dyn error::Error>> {
        let source = write_solidity_to_temp_file(source.to_string()).unwrap();
        let outdir = Builder::new()
            .prefix("gambit-compile-dir")
            .rand_bytes(5)
            .tempdir()?;
        let mut mutator = make_mutator(ops, source, outdir.into_path());
        let sources = mutator.sources().clone();
        mutator.mutate(sources)?;

        Ok(mutator)
    }

    /// Create a mutator for a single file, creating required components (e.g.,
    /// Solc, creating Sources and rapping them in a Vec<Rc<Source>>, etc)
    fn make_mutator(ops: &Vec<MutationType>, filename: PathBuf, outdir: PathBuf) -> Mutator {
        let conf = MutatorConf {
            mutation_operators: ops.clone(),
            funcs_to_mutate: None,
            contract: None,
        };
        let sourceroot = filename.parent().unwrap();

        let source = Source::new(filename.clone(), sourceroot.to_path_buf())
            .expect(format!("Could not build source from {}", filename.display()).as_str());
        let sources = vec![Rc::new(source)];
        let solc = Solc::new("solc".into(), PathBuf::from(outdir));
        Mutator::new(conf, sources, solc)
    }
}
