use crate::{SolAST, Source};
use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use std::{error, fmt::Display, rc::Rc, string::FromUtf8Error};

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

    pub fn as_source_file(&self) -> Result<String, FromUtf8Error> {
        let contents = self.source.contents();
        let prelude = &contents[0..self.start];
        let postlude = &contents[self.end..contents.len()];

        let res = [prelude, &self.repl.as_bytes(), postlude].concat();
        String::from_utf8(res)
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
    /// TODO: document this mutation operator
    AssignmentMutation,

    /// TODO: document this mutation operator
    BinaryOpMutation,

    /// TODO: document this mutation operator
    DeleteExpressionMutation,

    /// TODO: document this mutation operator
    ElimDelegateMutation,

    /// TODO: document this mutation operator
    FunctionCallMutation,

    /// TODO: document this mutation operator
    IfStatementMutation,

    /// TODO: document this mutation operator
    RequireMutation,

    /// TODO: document this mutation operator
    SwapArgumentsFunctionMutation,

    /// TODO: document this mutation operator
    SwapArgumentsOperatorMutation,

    /// TODO: document this mutation operator
    UnaryOperatorMutation,
}

impl ToString for MutationType {
    fn to_string(&self) -> String {
        let str = match self {
            MutationType::AssignmentMutation => "AssignmentMutation",
            MutationType::BinaryOpMutation => "BinaryOpMutation",
            MutationType::DeleteExpressionMutation => "DeleteExpressionMutation",
            MutationType::ElimDelegateMutation => "ElimDelegateMutation",
            MutationType::FunctionCallMutation => "FunctionCallMutation",
            MutationType::IfStatementMutation => "IfStatementMutation",
            MutationType::RequireMutation => "RequireMutation",
            MutationType::SwapArgumentsFunctionMutation => "SwapArgumentsFunctionMutation",
            MutationType::SwapArgumentsOperatorMutation => "SwapArgumentsOperatorMutation",
            MutationType::UnaryOperatorMutation => "UnaryOperatorMutation",
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
            MutationType::BinaryOpMutation => {
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
            MutationType::IfStatementMutation => {
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
            MutationType::UnaryOperatorMutation => {
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
                    } else if &kind == "number" {
                        vec!["-1", "0", "1"]
                    } else {
                        vec!["0"]
                    }
                } else {
                    vec!["0"]
                }
                .iter()
                .filter(|v| *v != &orig)
                .map(|v| *v)
                .collect();

                let (s, e) = rhs.get_bounds();
                replacements
                    .iter()
                    .map(|r| Mutant::new(source.clone(), self.clone(), s, e, r.to_string()))
                    .collect()
            }
            MutationType::BinaryOpMutation => {
                let orig = node.get_text(source.contents());
                let ops: Vec<&str> = vec!["+", "-", "*", "/", "%", "**"]
                    .iter()
                    .filter(|v| *v != &orig)
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

            MutationType::IfStatementMutation => {
                let cond = node.condition();
                let orig = node.get_text(source.contents());
                let bs: Vec<&str> = vec!["true", "false"]
                    .iter()
                    .filter(|v| *v != &orig)
                    .map(|v| *v)
                    .collect();

                let (start, end) = cond.get_bounds();

                bs.iter()
                    .map(|r| Mutant::new(source.clone(), self.clone(), start, end, r.to_string()))
                    .collect()
            }

            MutationType::RequireMutation => {
                let arg = &node.arguments()[0];
                let (start, end) = arg.get_bounds();
                vec![Mutant::new(
                    source.clone(),
                    self.clone(),
                    start,
                    end,
                    "!(".to_string() + &arg.get_text(source.contents()) + ")",
                )]
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

            MutationType::UnaryOperatorMutation => {
                let prefix_ops = vec!["++", "--", "~"];
                let suffix_ops = vec!["++", "--"];
                let (start, end) = node.get_bounds();
                let op = node
                    .operator()
                    .expect("Unary operation must have an operator!");
                let is_prefix = source.contents()[start] == op.as_bytes()[0];

                let replacements: Vec<&str> = if is_prefix { prefix_ops } else { suffix_ops }
                    .iter()
                    .filter(|v| *v != &op)
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
            MutationType::BinaryOpMutation,
            MutationType::DeleteExpressionMutation,
            MutationType::ElimDelegateMutation,
            MutationType::FunctionCallMutation,
            MutationType::IfStatementMutation,
            MutationType::RequireMutation,
            // MutationType::SwapArgumentsFunctionMutation,
            MutationType::SwapArgumentsOperatorMutation,
            MutationType::UnaryOperatorMutation,
        ]
    }
}
