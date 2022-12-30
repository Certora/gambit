use itertools::Itertools;
use rand::seq::SliceRandom;
use scanner_rust::Scanner;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

use crate::{
    ast, get_indent, get_path_normals, invoke_command, mutation, vec_pair_to_map, Mutation,
    MutationType::{self},
    SolAST,
};

/// How many tries for generating mutants.
static ATTEMPTS: i64 = 50;

/// Data structure for running mutations.
pub struct RunMutations {
    pub fnm: String,
    pub node: SolAST,
    pub num_mutants: i64,
    pub rand: rand_pcg::Pcg64,
    pub out: PathBuf,
    pub mutation_types: Vec<MutationType>,
    pub funcs_to_mutate: Option<Vec<String>>,
    pub contract: Option<String>,
}

impl RunMutations {
    /// Check if a node in the AST is an assert.
    pub fn is_assert_call(node: &SolAST) -> bool {
        node.name().map_or_else(|| false, |n| n == "assert")
    }

    /// Check that the path exists.
    fn lkup_mutant_dir(&self) -> PathBuf {
        let norm_path = get_path_normals(&self.fnm);
        let mut_dir = PathBuf::from(&self.out).join(norm_path);
        if mut_dir.parent().is_none() {
            panic!("{:?} does not exist", mut_dir);
        } else {
            mut_dir
        }
    }

    /// Returns the closures for visiting, accepting, and skipping AST nodes.
    fn mk_closures(
        mutation_types: Vec<MutationType>,
        funcs_to_mutate: Option<Vec<String>>,
        contract: Option<String>,
    ) -> (
        impl FnMut(&SolAST) -> Option<Vec<(mutation::MutationType, ast::SolAST)>>,
        impl Fn(&SolAST) -> bool,
        impl Fn(&SolAST) -> bool,
    ) {
        let visitor = move |node: &ast::SolAST| {
            let mapping: Vec<(mutation::MutationType, ast::SolAST)> = mutation_types
                .iter()
                .filter(|m| m.is_mutation_point(node))
                .map(|m| (*m, node.clone()))
                .into_iter()
                .collect();
            if mapping.is_empty() {
                None
            } else {
                Some(mapping)
            }
        };
        let skip = Self::is_assert_call;
        let accept = move |node: &SolAST| match (&contract, &funcs_to_mutate) {
            (None, None) => true,
            (Some(c), None) => node.contract.as_ref().map_or_else(|| false, |n| n.eq(c)),
            (None, Some(f)) => {
                node.node_type()
                    .map_or_else(|| false, |n| n == "FunctionDefinition")
                    && f.contains(&node.name().unwrap())
            }
            (Some(c), Some(f)) => {
                node.contract.as_ref().map_or_else(|| false, |n| n.eq(c))
                    && node
                        .node_type()
                        .map_or_else(|| false, |n| n == "FunctionDefinition")
                    && f.contains(&node.name().unwrap())
            }
        };
        (visitor, skip, accept)
    }

    /// Inner loop of mutation generation that uniformly
    /// genrates mutants from each possible mutation kind.
    fn inner_loop(
        mut_dir: PathBuf,
        fnm: String,
        num_mutants: i64,
        mut rand: rand_pcg::Pcg64,
        mut is_valid: impl FnMut(&str) -> bool,
        mutation_points: HashMap<MutationType, Vec<SolAST>>,
        mut mutation_points_todo: VecDeque<MutationType>,
    ) -> Vec<PathBuf> {
        let mut source = Vec::new();
        let orig_path = Path::new(&fnm);
        let mut f = File::open(orig_path).expect("File cannot be opened.");
        f.read_to_end(&mut source)
            .expect("Cannot read from file {}.");
        let source_to_str = std::str::from_utf8(&source)
            .expect("Cannot convert byte slice to string!")
            .into();
        let mut attempts = 0;
        let mut mutants: Vec<PathBuf> = vec![];
        let mut seen: HashSet<String> = HashSet::new();
        seen.insert(source_to_str);
        while !mutation_points_todo.is_empty() && attempts < num_mutants * ATTEMPTS {
            let mut_type = mutation_points_todo.remove(0).unwrap();
            let points = mutation_points
                .get(&mut_type)
                .expect("Found unexpected mutation.");
            if let Some(point) = points.choose(&mut rand) {
                let mut mutant = mut_type.mutate_randomly(point, &source, &mut rand);
                mutant = Self::add_mutant_comment(orig_path, &mutant, &mut_type);
                if !seen.contains(&mutant) && is_valid(&mutant) {
                    let mut_file =
                        mut_dir.to_str().unwrap().to_owned() + &attempts.to_string() + ".sol";
                    let mut_path = Path::new(&mut_file);
                    log::info!("attempting to write to {:?}", mut_path);
                    std::fs::write(mut_path, &mutant).expect("Failed to write mutant to file.");
                    Self::diff_mutant(orig_path, mut_path);
                    mutants.push(mut_path.to_owned());
                } else {
                    mutation_points_todo.push_back(mut_type);
                }
                seen.insert(mutant);
                attempts += 1;
            }
        }
        mutants
    }

    /// Logs the diff of the mutants w.r.t. the origin program.
    fn diff_mutant(orig: &Path, mutant: &Path) {
        let (succ, diff, _) = invoke_command(
            "diff",
            vec![orig.to_str().unwrap(), mutant.to_str().unwrap()],
        );
        match succ.unwrap_or_else(|| panic!("diff call terminated with a signal.")) {
            0 => log::info!("mutant identical to original program"),
            1 => log::info!("{}", std::str::from_utf8(&diff).unwrap()),
            _ => log::info!("install a `diff` program to see the diff"),
        }
    }

    /// Adds a comment to indicate what kind of mutation happened.
    fn add_mutant_comment(src_path: &Path, mutant: &String, mut_type: &MutationType) -> String {
        let mut scan1 =
            Scanner::scan_path(src_path).unwrap_or_else(|_| panic!("Cannot scan source path."));
        let mut scan2 = Scanner::new(mutant.as_bytes());
        let mut res = vec![];
        loop {
            let l1 = scan1.next_line_raw().unwrap();
            let l2 = scan2.next_line_raw().unwrap();
            if l1.is_none() || l2.is_none() {
                break;
            }
            let l1_to_str = String::from_utf8(l1.unwrap()).unwrap() + "\n";
            let l2_to_str = String::from_utf8(l2.unwrap()).unwrap() + "\n";
            if l1_to_str != l2_to_str {
                let indent = get_indent(&l1_to_str);
                let comment = indent + "/// " + &mut_type.to_string() + " of: " + l1_to_str.trim();
                res.push(comment);
                res.push("\n".to_string() + &l2_to_str);
                break;
            }
            res.push(l2_to_str);
        }
        loop {
            let l2 = scan2.next_line_raw().unwrap();
            if l2.is_none() {
                break;
            }
            let l2_to_str = String::from_utf8(l2.unwrap()).unwrap() + "\n";
            res.push(l2_to_str);
        }
        res.concat()
    }

    /// Mutation Generator that traverses the AST and determines which points
    /// can be mutated using which mutation type,
    /// then collects all the mutations that need to be done and calls
    /// `inner_loop` where the actual mutations are done.
    pub fn get_mutations(self, is_valid: impl FnMut(&str) -> bool) -> Vec<PathBuf> {
        let mut_dir = self.lkup_mutant_dir();
        let (visitor, skip, accept) =
            Self::mk_closures(self.mutation_types, self.funcs_to_mutate, self.contract);
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
            let points_len = points.len() as i64;
            let mutation_points = vec_pair_to_map(&mutations, &points);
            let mut mutation_points_todo = VecDeque::new();
            let mut remaining = self.num_mutants;
            while remaining > 0 {
                let to_take = std::cmp::min(remaining, points_len);
                let selected: Vec<&MutationType> = points.iter().take(to_take as usize).collect();
                for s in selected {
                    mutation_points_todo.push_back(*s);
                }
                //remaining -= points_len;
                remaining -= 1;
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
