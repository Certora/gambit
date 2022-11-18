use crate::SolAST;
use rand_pcg::*;

pub trait Mutation {
    fn is_mutation_point(node: SolAST, mut_type: MutationType) -> bool;
    fn mutate_randomly(
        node: SolAST,
        source: &[u8],
        rand: &mut Pcg64,
        mut_type: MutationType,
    ) -> String;
}

pub enum MutationType {
    BinaryOpMutation,
    RequireMutation,
}

impl Mutation for MutationType {
    fn is_mutation_point(node: SolAST, mut_type: MutationType) -> bool {
        match mut_type {
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
        return false;
    }

    fn mutate_randomly(
        node: SolAST,
        source: &[u8],
        rand: &mut Pcg64,
        mut_type: MutationType,
    ) -> String {
        match mut_type {
            MutationType::BinaryOpMutation => todo!(),
            MutationType::RequireMutation => todo!(),
        }
    }
}
