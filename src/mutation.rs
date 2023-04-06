use crate::{get_indent, SolAST, Source};
use clap::ValueEnum;
use serde::{Deserialize, Serialize};
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
    /// TODO: Cache these contents
    pub fn as_source_file(&self) -> Result<String, Box<dyn error::Error>> {
        let contents = self.source.contents();
        let prelude = &contents[0..self.start];
        let postlude = &contents[self.end..contents.len()];

        let res = [prelude, &self.repl.as_bytes(), postlude].concat();
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

/// Every kind of mutation implements this trait.
///
/// TODO: Document
pub trait Mutation {
    /// Check if this mutation applies to this AST node
    fn applies_to(&self, node: &SolAST) -> bool;

    /// Generate all mutants of a given node by this agent
    fn mutate(&self, node: &SolAST, source: Rc<Source>) -> Vec<Mutant>;
}

/// Kinds of mutations.
#[derive(Hash, Eq, PartialEq, Clone, Copy, Debug, ValueEnum, Deserialize, Serialize)]
pub enum MutationType {
    AssignmentMutation,
    BinOpMutation,
    DeleteExpressionMutation,
    ElimDelegateMutation,
    FunctionCallMutation,
    IfCondMutation,
    RequireMutation,
    SwapArgumentsFunctionMutation,
    SwapArgumentsOperatorMutation,
    UnOpMutation,
}

impl ToString for MutationType {
    fn to_string(&self) -> String {
        let str = match self {
            MutationType::AssignmentMutation => "AssignmentMutation",
            MutationType::BinOpMutation => "BinaryOpMutation",
            MutationType::DeleteExpressionMutation => "DeleteExpressionMutation",
            MutationType::ElimDelegateMutation => "ElimDelegateMutation",
            MutationType::FunctionCallMutation => "FunctionCallMutation",
            MutationType::IfCondMutation => "IfStatementMutation",
            MutationType::RequireMutation => "RequireMutation",
            MutationType::SwapArgumentsFunctionMutation => "SwapArgumentsFunctionMutation",
            MutationType::SwapArgumentsOperatorMutation => "SwapArgumentsOperatorMutation",
            MutationType::UnOpMutation => "UnaryOperatorMutation",
        };
        str.into()
    }
}

impl Mutation for MutationType {
    fn applies_to(&self, node: &SolAST) -> bool {
        match self {
            MutationType::AssignmentMutation => {
                if let Some(n) = node.node_type() {
                    return n == "Assignment";
                }
            }
            MutationType::BinOpMutation => {
                if let Some(n) = node.node_type() {
                    return n == "BinaryOperation";
                }
            }
            MutationType::DeleteExpressionMutation => {
                if let Some(n) = node.node_type() {
                    return n == "ExpressionStatement";
                }
            }
            MutationType::ElimDelegateMutation => {
                return node.node_type().map_or_else(
                    || false,
                    |n| {
                        n == "FunctionCall"
                            && (node
                                .expression()
                                .node_type()
                                .map_or_else(|| false, |nt| nt == "MemberAccess"))
                            && (node
                                .expression()
                                .get_string("memberName")
                                .map_or_else(|| false, |mn| mn == "delegatecall"))
                    },
                );
            }
            MutationType::FunctionCallMutation => {
                if let Some(n) = node.node_type() {
                    return n == "FunctionCall" && !node.arguments().is_empty();
                }
            }
            MutationType::IfCondMutation => {
                if let Some(n) = node.node_type() {
                    return n == "IfStatement";
                }
            }
            MutationType::RequireMutation => {
                return node.node_type().map_or_else(
                    || false,
                    |n| {
                        n == "FunctionCall"
                            && (node
                                .expression()
                                .name()
                                .map_or_else(|| false, |nm| nm == "require"))
                            && !node.arguments().is_empty()
                    },
                );
            }
            MutationType::SwapArgumentsFunctionMutation => {
                if let Some(n) = node.node_type() {
                    return n == "FunctionCall" && node.arguments().len() > 1;
                }
            }
            MutationType::SwapArgumentsOperatorMutation => {
                // TODO: should we include "/" and "%"? This might create a trivial mutant (e.g., a / 0), which is not useful!
                let non_comm_ops = vec!["-", "/", "%", "**", ">", "<", ">=", "<=", "<<", ">>"];
                if let Some(n) = node.node_type() {
                    return n == "BinaryOperation"
                        && non_comm_ops.contains(
                            &node
                                .operator()
                                .unwrap_or_else(|| panic!("Expression does not have operator"))
                                .as_str(),
                        );
                }
            }
            MutationType::UnOpMutation => {
                if let Some(n) = node.node_type() {
                    return n == "UnaryOperation";
                }
            }
        }
        false
    }

    /// Produce all mutants at the given node
    ///
    /// # Arguments
    ///
    /// * `node` - The Solidity AST node to mutate
    /// * `source` - The original source file: we use this to generate a new
    ///   source file
    fn mutate(&self, node: &SolAST, source: Rc<Source>) -> Vec<Mutant> {
        if !self.applies_to(node) {
            return vec![];
        }
        match self {
            MutationType::AssignmentMutation => {
                let rhs = node.right_hand_side();
                let node_kind = rhs.node_kind();
                let orig = rhs.get_text(source.contents());
                let replacements: Vec<&str> = if let Some(kind) = node_kind {
                    if &kind == "bool" {
                        vec!["true", "false"]
                    } else if rhs.is_literal_number() {
                        vec!["(-1)", "0", "1"]
                    } else {
                        vec!["0"]
                    }
                } else {
                    vec!["0"]
                }
                .iter()
                .filter(|v| !orig.eq(*v))
                .map(|v| *v)
                .collect();

                let (s, e) = rhs.get_bounds();
                replacements
                    .iter()
                    .map(|r| Mutant::new(source.clone(), self.clone(), s, e, r.to_string()))
                    .collect()
            }
            MutationType::BinOpMutation => {
                let orig = node.operator().unwrap();
                let orig = String::from(orig.trim());

                let ops: Vec<&str> = vec!["+", "-", "*", "/", "%", "**"]
                    .iter()
                    .filter(|v| !orig.eq(*v))
                    .map(|v| *v)
                    .collect();

                let (_, endl) = node.left_expression().get_bounds();
                let (startr, _) = node.right_expression().get_bounds();
                ops.iter()
                    .map(|op| {
                        Mutant::new(source.clone(), self.clone(), endl, startr, op.to_string())
                    })
                    .collect()
            }

            // TODO: Delete expression or delete a statement?
            MutationType::DeleteExpressionMutation => {
                let (start, end) = node.get_bounds();
                vec![Mutant::new(
                    source.clone(),
                    self.clone(),
                    start,
                    end,
                    ";".into(),
                )]
            }
            MutationType::ElimDelegateMutation => {
                let (_, endl) = node.expression().expression().get_bounds();
                let (_, endr) = node.expression().get_bounds();

                vec![Mutant::new(
                    source,
                    self.clone(),
                    endl + 1,
                    endr,
                    "call".to_string(),
                )]
            }

            // TODO: Should we enable this? I'm not sure if this is the best mutation operator
            MutationType::FunctionCallMutation => {
                // if let Some(arg) = node.arguments().choose(rand) {
                //     node.replace_in_source(source, arg.get_text(source))
                // } else {
                //     node.get_text(source)
                // }

                vec![] // For now I'm removing this operator: not sure what it does!
            }

            MutationType::IfCondMutation => {
                let cond = node.condition();
                let orig = cond.get_text(source.contents());
                let bs: Vec<&str> = vec!["true", "false"]
                    .iter()
                    .filter(|v| !orig.eq(*v))
                    .map(|v| *v)
                    .collect();

                let (start, end) = cond.get_bounds();

                bs.iter()
                    .map(|r| Mutant::new(source.clone(), self.clone(), start, end, r.to_string()))
                    .collect()
            }

            MutationType::RequireMutation => {
                let arg = &node.arguments()[0];
                let orig = arg.get_text(source.contents());
                let bs: Vec<&str> = vec!["true", "false"]
                    .iter()
                    .filter(|v| !orig.eq(*v))
                    .map(|v| *v)
                    .collect();
                let (start, end) = arg.get_bounds();
                bs.iter()
                    .map(|r| Mutant::new(source.clone(), self.clone(), start, end, r.to_string()))
                    .collect()
            }

            MutationType::SwapArgumentsFunctionMutation => {
                vec![]

                // TODO: I'm removing this operator for now as I'm not sure how
                // to implement it deterministically. I'm also faily convinced
                // that this operator should be removed

                // let mut children = node.arguments();
                // children.shuffle(rand);

                // if children.len() == 2 {
                //     node.replace_multiple(
                //         source,
                //         vec![
                //             (children[0].clone(), children[1].get_text(source)),
                //             (children[1].clone(), children[0].get_text(source)),
                //         ],
                //     )
                // } else {
                //     node.get_text(source)
                // }
            }

            MutationType::SwapArgumentsOperatorMutation => {
                // TODO: I've removed this for now since I think this is highly likely to be equivalent
                vec![]
            }

            MutationType::UnOpMutation => {
                let prefix_ops = vec!["++", "--", "~"];
                let suffix_ops = vec!["++", "--"];

                let op = node
                    .operator()
                    .expect("Unary operation must have an operator!");

                let (start, end) = node.get_bounds();
                let is_prefix = source.contents()[start] == op.as_bytes()[0];
                let replacements: Vec<&str> = if is_prefix { prefix_ops } else { suffix_ops }
                    .iter()
                    .filter(|v| !op.eq(*v))
                    .map(|v| *v)
                    .collect();
                let (start, end) = if is_prefix {
                    (start, start + op.len())
                } else {
                    (end - op.len(), end)
                };

                replacements
                    .iter()
                    .map(|r| Mutant::new(source.clone(), self.clone(), start, end, r.to_string()))
                    .collect()
            }
        }
    }
}

impl MutationType {
    pub fn default_mutation_operators() -> Vec<MutationType> {
        vec![
            MutationType::AssignmentMutation,
            MutationType::BinOpMutation,
            MutationType::DeleteExpressionMutation,
            MutationType::ElimDelegateMutation,
            MutationType::FunctionCallMutation,
            MutationType::IfCondMutation,
            MutationType::RequireMutation,
            // MutationType::SwapArgumentsFunctionMutation,
            MutationType::SwapArgumentsOperatorMutation,
            MutationType::UnOpMutation,
        ]
    }
}

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
        assert_exact_mutants(&vec!["uint256 x;", "x = 3;"], &ops, &vec!["(-1)", "0", "1"]);
        assert_exact_mutants(&vec!["int256 x;", "x = 1;"], &ops, &vec!["(-1)", "0"]);
        assert_exact_mutants(&vec!["int256 x;", "x = 0;"], &ops, &vec!["(-1)", "1"]);
        // FIXME: The following three test cases are BROKEN!! Currently these
        // all get mutated to '0' because they are not 'number's
        assert_num_mutants(&vec!["int256 x;", "x = -2;"], &vec![AssignmentMutation], 1);
        assert_num_mutants(&vec!["int256 x;", "x = -1;"], &vec![AssignmentMutation], 1);
        assert_num_mutants(&vec!["int256 x;", "x = -0;"], &vec![AssignmentMutation], 1);

        assert_exact_mutants(&vec!["bool b;", "b = true;"], &ops, &vec!["false"]);
        assert_exact_mutants(&vec!["bool b;", "b = false;"], &ops, &vec!["true"]);

        Ok(())
    }

    #[test]
    pub fn test_binary_op_mutation() -> Result<(), Box<dyn error::Error>> {
        let ops = vec![BinOpMutation];
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
        assert_exact_mutants(&vec!["uint256 x = 1 + 2;"], &ops, &without("+"));
        assert_exact_mutants(&vec!["uint256 x = 1 - 2;"], &ops, &without("-"));
        assert_exact_mutants(&vec!["uint256 x = 1 * 2;"], &ops, &without("*"));
        assert_exact_mutants(&vec!["uint256 x = 1 / 2;"], &ops, &without("/"));
        assert_exact_mutants(&vec!["uint256 x = 1 % 2;"], &ops, &without("%"));
        assert_exact_mutants(&vec!["uint256 x = 1 ** 2;"], &ops, &without("**"));
        Ok(())
    }

    #[test]
    pub fn test_delete_expression_mutation() -> Result<(), Box<dyn error::Error>> {
        let ops = vec![DeleteExpressionMutation];
        assert_exact_mutants(&vec!["foo();"], &ops, &vec![";"]);
        assert_exact_mutants(&vec!["x = 3;"], &ops, &vec![";"]);
        Ok(())
    }

    #[test]
    pub fn test_elim_delegate_mutation() -> Result<(), Box<dyn error::Error>> {
        let _ops = vec![ElimDelegateMutation];
        // TODO: how should I test this?
        Ok(())
    }

    #[test]
    pub fn test_function_call_mutation() -> Result<(), Box<dyn error::Error>> {
        let _ops = vec![FunctionCallMutation];
        // TODO: how should I test this?
        Ok(())
    }

    #[test]
    pub fn test_if_statement_mutation() -> Result<(), Box<dyn error::Error>> {
        let ops = vec![IfCondMutation];
        assert_num_mutants(&vec!["if (true) { x = 1; } else { x = 2 ;}"], &ops, 1);
        assert_num_mutants(&vec!["if (true) {}"], &ops, 1);
        Ok(())
    }

    #[test]
    pub fn test_require_mutation() -> Result<(), Box<dyn error::Error>> {
        let ops = vec![RequireMutation];
        assert_num_mutants(&vec!["require(c);"], &ops, 2);
        assert_num_mutants(&vec!["require(true);"], &ops, 1);
        assert_num_mutants(&vec!["require(a && b);"], &ops, 2);
        Ok(())
    }

    #[test]
    pub fn test_unary_op_mutation() -> Result<(), Box<dyn error::Error>> {
        let ops = vec![UnOpMutation];
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
        assert_exact_mutants(&vec!["uint256 x = ++a;"], &ops, &without_prefix("++"));
        assert_exact_mutants(&vec!["uint256 x = --a;"], &ops, &without_prefix("--"));
        assert_exact_mutants(&vec!["uint256 x = ~a;"], &ops, &without_prefix("~"));
        assert_exact_mutants(&vec!["uint256 x = a--;"], &ops, &without_suffix("--"));
        assert_exact_mutants(&vec!["uint256 x = a++;"], &ops, &without_suffix("++"));
        Ok(())
    }

    fn assert_exact_mutants(statements: &Vec<&str>, ops: &Vec<MutationType>, expected: &Vec<&str>) {
        let mutator = apply_mutation(statements, None, ops).unwrap();
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

    fn assert_num_mutants(statements: &Vec<&str>, ops: &Vec<MutationType>, expected: usize) {
        let mutator = apply_mutation(statements, None, ops).unwrap();
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

    fn apply_mutation(
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
        mutator.mutate()?;

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

        let source = Source::new(filename).unwrap();
        let sources = vec![Rc::new(source)];
        let solc = Solc::new("solc".into(), PathBuf::from(outdir));
        Mutator::new(conf, sources, solc)
    }
}
