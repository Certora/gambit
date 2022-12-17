use itertools::Itertools;
use rand::seq::SliceRandom;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    fs::{self, File},
    io::Read,
    path::PathBuf,
    str::FromStr,
};

use crate::{
    ast, get_path_normals, mutation, vec_pair_to_map, Mutation,
    MutationType::{self},
    SolAST,
};

/// How many tries for generating mutants.
static ATTEMPTS: i32 = 50;

/// Data structure for running mutations.
pub struct RunMutations {
    pub fnm: String,
    pub node: SolAST,
    pub num_mutants: i32,
    pub rand: rand_pcg::Pcg64,
    pub out: PathBuf,
    pub mutation_types: Vec<MutationType>,
}

impl RunMutations {
    pub fn new(
        fnm: String,
        node: SolAST,
        num_mutants: i32,
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
            mutation_types: muts,
        }
    }

    /// Check if a node in the AST is an assert.
    pub fn is_assert_call(node: &SolAST) -> bool {
        node.name().map_or_else(|| false, |n| n == "assert")
    }

    fn mk_mutant_dir(&self) -> PathBuf {
        let norm_path = get_path_normals(&self.fnm);
        let mut_dir = self.out.join(norm_path);
        if let Some(pd) = mut_dir.parent() {
            if pd.is_dir() {
                fs::remove_dir_all(pd)
                    .expect("Directory existed but was unable to remove content.");
            }
        }
        std::fs::create_dir_all(mut_dir.parent().unwrap())
            .expect("Unable to create output directory.");
        mut_dir
    }

    pub fn mk_closures(
        mutation_types: Vec<MutationType>,
    ) -> (
        impl FnMut(&SolAST) -> Option<Vec<(mutation::MutationType, ast::SolAST)>>,
        impl Fn(&SolAST) -> bool,
        impl Fn(&SolAST) -> bool,
    ) {
        let visitor = move |node: &ast::SolAST| {
            let mapping: Vec<(mutation::MutationType, ast::SolAST)> = mutation_types
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
        let accept = |_: &SolAST| true; // node.node_type().map_or_else(|| false, |n| n == *"FunctionDefinition".to_string())
        (visitor, skip, accept)
    }

    fn inner_loop(
        mut_dir: PathBuf,
        fnm: String,
        num_mutants: i32,
        mut rand: rand_pcg::Pcg64,
        mut is_valid: impl FnMut(&str) -> bool,
        mutation_points: HashMap<MutationType, Vec<SolAST>>,
        mut mutation_points_todo: VecDeque<MutationType>,
    ) -> Vec<PathBuf> {
        let mut source = Vec::new();
        let mut f = File::open(fnm).expect("File cannot be opened.");
        f.read_to_end(&mut source)
            .expect("Cannot read from file {}.");
        let source_to_str = std::str::from_utf8(&source)
            .expect("Cannot convert byte slice to string!")
            .to_string();
        let mut attempts = 0;
        let mut mutants: Vec<PathBuf> = vec![];
        let mut seen: HashSet<String> = HashSet::new();
        seen.insert(source_to_str);
        while !mutation_points_todo.is_empty() && attempts < num_mutants * ATTEMPTS {
            let mutation = mutation_points_todo.remove(0).unwrap();
            let points = mutation_points
                .get(&mutation)
                .expect("Found unexpected mutation.");
            if let Some(point) = points.choose(&mut rand) {
                let mutant = mutation.mutate_randomly(point, &source, &mut rand);
                if !seen.contains(&mutant) && is_valid(&mutant) {
                    let mut_file =
                        mut_dir.to_str().unwrap().to_owned() + &attempts.to_string() + ".sol";
                    log::info!("attempting to write to {}", mut_file);
                    std::fs::write(&mut_file, &mutant).expect("Failed to write mutant to file.");
                    mutants.push(
                        PathBuf::from_str(&mut_file)
                            .unwrap_or_else(|_| panic!("Failed to add mutant path to mutants")),
                    );
                } else {
                    mutation_points_todo.push_back(mutation);
                }
                seen.insert(mutant);
                attempts += 1;
            }
        }
        mutants
    }

    pub fn get_mutations(self, is_valid: impl FnMut(&str) -> bool) -> Vec<PathBuf> {
        let mut_dir = self.mk_mutant_dir();
        let (visitor, skip, accept) = Self::mk_closures(self.mutation_types);
        let mutations: Vec<(MutationType, SolAST)> = self
            .node
            .traverse(visitor, skip, accept)
            .into_iter()
            .flatten()
            .collect();
        if !mutations.is_empty() {
            let (mut points, _): (Vec<MutationType>, Vec<SolAST>) =
                mutations.iter().cloned().unzip();
            points = points.into_iter().unique().collect();
            let points_len = points.len() as i32;
            let mutation_points = vec_pair_to_map(mutations, &points);
            let mut mutation_points_todo = VecDeque::new();
            let mut remaining = self.num_mutants;
            while remaining > 0 {
                let to_take = std::cmp::min(remaining, points_len);
                let selected: Vec<&MutationType> = points.iter().take(to_take as usize).collect();
                for s in selected {
                    mutation_points_todo.push_back(s.clone());
                }
                remaining -= points_len;
            }
            Self::inner_loop(
                mut_dir,
                self.fnm,
                self.num_mutants,
                self.rand,
                is_valid,
                mutation_points,
                mutation_points_todo,
            )
        } else {
            log::info!("Did not find any mutations");
            vec![]
        }
    }
}
