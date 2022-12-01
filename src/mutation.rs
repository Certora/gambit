use std::str::FromStr;

use crate::SolAST;
use rand_pcg::*;

/// Every kind of mutation implements this trait.
pub trait Mutation {
    fn is_mutation_point(&self, node: &SolAST) -> bool;
    fn mutate_randomly(&self, node: &SolAST, source: &[u8], rand: &mut Pcg64) -> String;
}

/// Kinds of mutations.
#[derive(Hash, Eq, PartialEq, Clone, Debug)]
pub enum MutationType {
    BinaryOpMutation,
    RequireMutation,
}

impl FromStr for MutationType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "BinaryOpMutation" => Ok(MutationType::BinaryOpMutation),
            "RequireMutation" => Ok(MutationType::RequireMutation),
            _ => panic!("Undefined mutant!"),
        }
    }
}

impl Mutation for MutationType {
    fn is_mutation_point(&self, node: &SolAST) -> bool {
        match self {
            MutationType::BinaryOpMutation => {
                if let Some(n) = node.node_type() {
                    return n == "BinaryOperation";
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
        }
        false
    }

    fn mutate_randomly(&self, node: &SolAST, source: &[u8], _rand: &mut Pcg64) -> String {
        match self {
            MutationType::BinaryOpMutation => {
                assert!(&self.is_mutation_point(node));
                let (_, endl) = node.left_expression().get_bounds();
                let (startr, _) = node.right_expression().get_bounds();
                // TODO: actually do this randomly!
                // log::info!("mutating {:?}", String::from_utf8(source.to_vec()));
                node.replace_part(source, " ".to_string() + "-" + " ", endl, startr)
            }
            MutationType::RequireMutation => {
                assert!(&self.is_mutation_point(node));
                let arg = &node.arguments()[0];
                arg.replace_in_source(source, "!(".to_string() + &arg.get_text(source) + ")")
            }
        }
    }
}
