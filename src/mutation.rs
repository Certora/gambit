use crate::SolAST;
use clap::ValueEnum;
use serde::{Deserialize, Serialize};

/// Every kind of mutation implements this trait.
///
/// `is_mutation_point` determines whether a node in the AST
/// is a valid node for performing a certain `MutationType`.
///
/// `mutate_randomly` mutates such nodes by randomly selecting
/// one of many possible ways to perform `MutationType`.
///
/// For example, consider the `BinaryOpMutation` `MutationType`.
/// The method `is_mutation_point` for this mutation checks where the
/// node under question has the `node_type` `BinaryOperation`.
///
/// `mutate_randomly` for this mutation will randomly pick one
/// of many binary operators supported in Solidity (e.g., +, -, *, /, **, ...])
/// and apply it at the source location of the original binary operator.
///
pub trait Mutation {
    /// Check if this mutation applies to this AST node
    fn is_mutation_point(&self, node: &SolAST) -> bool;

    /// Generate all mutants of a given node by this agent
    fn mutate(&self, node: &SolAST, source: &[u8]) -> Vec<String>;
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

/// Apply a set of replacements to a node. This is a helper function used for
/// the `MutationType.mutate`: it iterates through all provided replacements and
/// replaces the occurence of the given AST node in source with each
/// replacement, returning a vec of mutants
fn apply_replacements(source: &[u8], node: &SolAST, replacements: &[&str]) -> Vec<String> {
    return replacements
        .iter()
        .map(|r| node.replace_in_source(source, r.to_string()))
        .collect();
}

/// Apply a set of replacements to a node with explicit bounds. This is a helper
/// function used for the `MutationType.mutate` for, e.g., operators where we
/// don't want to replace the entire node.
fn apply_replacements_to_bounds(
    source: &[u8],
    node: &SolAST,
    replacements: &[&str],
    start: usize,
    end: usize,
) -> Vec<String> {
    return replacements
        .iter()
        .map(|r| node.replace_part(source, r.to_string(), start, end))
        .collect();
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
    fn is_mutation_point(&self, node: &SolAST) -> bool {
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
    fn mutate(&self, node: &SolAST, source: &[u8]) -> Vec<String> {
        if !self.is_mutation_point(node) {
            return vec![];
        }
        match self {
            MutationType::AssignmentMutation => {
                let replacements = vec!["true", "false", "0", "1"];
                // TODO: filter replacements by mutation type

                let rhs = node.right_hand_side();
                apply_replacements(source, &rhs, &replacements)
            }
            MutationType::BinaryOpMutation => {
                let ops = vec!["+", "-", "*", "/", "%", "**"];
                // TODO: Check for types?
                let (_, endl) = node.left_expression().get_bounds();
                let (startr, _) = node.right_expression().get_bounds();
                apply_replacements_to_bounds(source, node, &ops, endl, startr)
            }

            // TODO: Delete expression or delete a statement?
            MutationType::DeleteExpressionMutation => vec![node.comment_out(source)],

            MutationType::ElimDelegateMutation => {
                let (_, endl) = node.expression().expression().get_bounds();
                let (_, endr) = node.expression().get_bounds();
                vec![node.replace_part(source, "call".to_string(), endl + 1, endr)]
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
                let bs = vec!["true", "false"];

                apply_replacements(source, &cond, &bs)
            }

            MutationType::RequireMutation => {
                let arg = &node.arguments()[0];
                vec![arg.replace_in_source(source, "!(".to_string() + &arg.get_text(source) + ")")]
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
                let left = node.left_expression();
                let right = node.right_expression();
                vec![node.replace_multiple(
                    source,
                    vec![
                        (left.clone(), right.get_text(source)),
                        (right, left.get_text(source)),
                    ],
                )]
            }

            MutationType::UnaryOperatorMutation => {
                let prefix_ops = vec!["++", "--", "~"];
                let suffix_ops = vec!["++", "--"];
                let (start, end) = node.get_bounds();
                let op = node
                    .operator()
                    .expect("Unary operation must have an operator!");
                let is_prefix = source[0] == op.as_bytes()[0];
                if is_prefix {
                    apply_replacements_to_bounds(source, node, &prefix_ops, start, start + op.len())
                } else {
                    apply_replacements_to_bounds(source, node, &suffix_ops, end - op.len(), end)
                }
            }
        }
    }
}
