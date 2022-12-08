use itertools::Itertools;
use rand::seq::SliceRandom;
use std::{
    collections::{HashMap, VecDeque},
    fs::File,
    io::Read,
    path::PathBuf,
};

use crate::{
    ast, get_path_normals, mutation, Mutation,
    MutationType::{self},
    SolAST,
};

/// How many tries for generating mutants.
static ATTEMPTS: usize = 50;

/// Data structure for running mutations.
pub struct RunMutations {
    pub fnm: String,
    pub node: SolAST,
    pub num_mutants: usize,
    pub rand: rand_pcg::Pcg64,
    pub out: PathBuf,
    pub mutations: Vec<MutationType>,
}

impl RunMutations {
    pub fn new(
        fnm: String,
        node: SolAST,
        num_mutants: usize,
        rand: rand_pcg::Pcg64,
        out: PathBuf,
        muts: Vec<MutationType>,
    ) -> Self {
        Self {
            fnm,
            node,
            num_mutants,
            rand,
            out,
            mutations: muts,
        }
    }

    /// Check if a node in the AST is an assert.
    pub fn is_assert_call(node: &SolAST) -> bool {
        node.name().map_or_else(|| false, |n| n == "assert")
    }

    pub fn get_mutations(mut self) -> Vec<PathBuf> {
        let visitor = |node: &ast::SolAST| {
            let mapping: Vec<(mutation::MutationType, ast::SolAST)> = self
                .mutations
                .iter()
                .filter(|m| m.is_mutation_point(node))
                .map(|m| (m.clone(), node.clone()))
                .into_iter()
                .collect();
            if mapping.is_empty() {
                None
            } else {
                Some(mapping)
            }
        };

        let skip = Self::is_assert_call;
        // TODO: add the case where we have specific functions from the user to mutate.
        // TODO: allow manual mutations too
        let accept = |_: &SolAST| true; // node.node_type().map_or_else(|| false, |n| n == *"FunctionDefinition".to_string())
        let mutations = self.node.traverse(visitor, skip, accept);
        if !mutations.is_empty() {
            log::info!("found {} mutations", mutations.len());
            let mut flatten: Vec<(MutationType, SolAST)> = vec![];
            for inner in mutations {
                for elem in inner {
                    flatten.push(elem);
                }
            }
            let (mut_types, _): (Vec<MutationType>, Vec<SolAST>) = flatten.iter().cloned().unzip();
            let mut_types: Vec<MutationType> = mut_types.iter().unique().cloned().collect();
            let mut mutation_points: HashMap<MutationType, Vec<SolAST>> = HashMap::new();

            for mutt in mut_types {
                let mut nodes = vec![];
                for (m, n) in &flatten {
                    if mutt == *m {
                        nodes.push(n.clone());
                    }
                }
                mutation_points.insert(mutt, nodes);
            }

            let mut mutation_points_todo = VecDeque::new();
            let point_list: Vec<MutationType> = mutation_points.clone().into_keys().collect();
            // TODO: check that point_list is not empty.
            let mut remaining = self.num_mutants;
            while remaining > 0 {
                let to_take = std::cmp::min(remaining, point_list.len());
                let selected: Vec<&MutationType> = point_list.iter().take(to_take).collect();
                for s in selected {
                    mutation_points_todo.push_back(s);
                }
                remaining -= point_list.len();
            }

            let mut source = Vec::new();
            let mut f = File::open(&self.fnm).expect("File cannot be opened.");
            f.read_to_end(&mut source)
                .expect("Cannot read from file {}.");

            let mut attempts = 0;
            let mut mutants: Vec<PathBuf> = vec![];
            let mut seen: Vec<String> = vec![];
            let source_to_str = std::str::from_utf8(&source)
                .expect("Cannot convert byte slice to string!")
                .to_string();
            seen.push(source_to_str);
            while !mutation_points_todo.is_empty() && attempts < self.num_mutants * ATTEMPTS {
                let mutation = mutation_points_todo.remove(0).unwrap();
                let points = &mutation_points
                    .get(mutation)
                    .expect("Found unexpected mutation.");
                if let Some(point) = points.choose(&mut self.rand) {
                    let mutant = mutation.mutate_randomly(point, &source, &mut self.rand);
                    let norm_path = get_path_normals(&self.fnm);
                    let mut_file = &self.out.join(
                        norm_path.to_str().unwrap().to_owned() + &attempts.to_string() + ".sol",
                    );
                    std::fs::create_dir_all(mut_file.parent().unwrap())
                        .expect("Unable to create output directory.");
                    log::info!("attempting to write to {}", mut_file.to_str().unwrap());
                    std::fs::write(mut_file, mutant.clone())
                        .expect("Failed to write mutant to file.");
                    if seen.contains(&mutant) {
                        // skip this mutant.
                    } else {
                        mutants.push(mut_file.to_path_buf());
                    }
                    // TODO: run the solidity compiler here to check that the mutant compiles.
                    seen.push(mutant);
                    attempts += 1;
                }
            }
            mutants
        } else {
            log::info!("Did not find any mutations");
            vec![]
        }
    }
}
