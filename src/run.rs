use scanner_rust::{Scanner, ScannerError};
use std::{
    collections::HashSet,
    error::Error,
    fs::File,
    io::{self, Read},
    path::{Path, PathBuf},
};

use crate::{
    ast, canon_path_from_str, get_indent, invoke_command, mutation, next_mid, Mutation,
    MutationType::{self},
    SolAST,
};

/// How many tries for generating mutants.
pub static MUTANTS_DIR: &str = "mutants";
static FUNCTIONDEFINITION: &str = "FunctionDefinition";
static DOT_SOL: &str = ".sol";

/// Data structure performing mutation generation on a single file
/// TODO: Document this
pub struct RunMutations {
    /// Name of file to mutate
    pub filename: String,
    /// Root node of mutation: all mutations will be performed on this node or on children nodes
    pub mutation_root: SolAST,
    /// Root of the output directory structure; defaults to `out/` in the
    /// current working directory
    pub out: PathBuf,
    /// Mutation operators to be applied
    pub mutation_types: Vec<MutationType>,
    /// If this is `Some(fnames)` then only mutate functions with names in
    /// `fnames`. If this is `None` then mutate all function names
    pub funcs_to_mutate: Option<Vec<String>>,
    /// If this is `Some(c)` then only mutate SolAST `ast` when `ast.contract ==
    /// c`. When this is `None` then no constraints are given.
    pub contract: Option<String>,
}

impl RunMutations {
    /// Check if a node in the AST is an assert.
    pub fn is_assert_call(node: &SolAST) -> bool {
        node.name().map_or_else(|| false, |n| n == "assert")
    }

    /// Check that the path exists.
    /// TODO: This doesn't seem to do any checking
    fn lkup_mutant_dir(&self) -> io::Result<PathBuf> {
        let norm_path = canon_path_from_str(&self.filename)?;
        let mut_dir =
            PathBuf::from(&self.out).join(MUTANTS_DIR.to_owned() + norm_path.to_str().unwrap());
        Ok(mut_dir)
    }

    /// Returns the closures for visiting, skipping, and accepting AST nodes.
    ///
    /// # Returns
    ///
    /// Returns a 3-tuple `(visitor, skip, accept)`:
    ///
    /// * `visitor` - a closure mapping AST node `node` to all `(mut_type, node)`
    ///   tuples where `mut_type` is applicable to `node` (i.e.,
    ///   `mut_type.is_mutation_point(node)`)
    /// * `skip` - a closure that returns `true` when a node should be skipped
    ///   TODO: what does skipping mean? Do we not recursively visit it?
    /// * `accept` - a closure that returns `true` when the node satisfies all
    ///   given mutation constraints:
    ///   1. the node belongs to a specified function if at least one function
    ///      name was given, and
    ///   2. the node's contract is the specified contract if one was given
    fn mk_closures(
        mutation_types: Vec<MutationType>,
        funcs_to_mutate: Option<Vec<String>>,
        contract: Option<String>,
    ) -> (
        impl FnMut(&SolAST) -> Option<Vec<(mutation::MutationType, ast::SolAST)>>,
        impl Fn(&SolAST) -> bool,
        impl Fn(&SolAST) -> bool,
    ) {
        // visitor: map an AST node to a vec of (MutationType, node)
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
                    .map_or_else(|| false, |n| n == FUNCTIONDEFINITION)
                    // TODO: is `f.contains` right? What if one function name is a substring of another?
                    && f.contains(&node.name().unwrap())
            }
            (Some(c), Some(f)) => {
                node.contract.as_ref().map_or_else(|| false, |n| n.eq(c))
                    && node
                        .node_type()
                        .map_or_else(|| false, |n| n == FUNCTIONDEFINITION)
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
        mut is_valid: impl FnMut(&str) -> Result<bool, Box<dyn std::error::Error>>,
        mutations: Vec<(MutationType, SolAST)>,
    ) -> Result<Vec<(PathBuf, serde_json::Value)>, Box<dyn Error>> {
        let mut source = Vec::new();
        let orig_path = Path::new(&fnm);
        let mut f = File::open(orig_path)?;
        f.read_to_end(&mut source)?;
        let source_to_str = std::str::from_utf8(&source)?.into();

        let mut mutants: Vec<(PathBuf, serde_json::Value)> = vec![];

        let mut seen: HashSet<String> = HashSet::new();
        seen.insert(source_to_str);

        for (mtype, node) in mutations {
            for m in mtype.mutate(&node, &source) {
                if seen.contains(&m) || !is_valid(&m)? {
                    continue;
                }
                seen.insert(m.clone());
                // TODO: what happens if we can't add a comment?
                let m = if let Ok(res) = Self::add_mutant_comment(orig_path, &m, &mtype) {
                    res
                } else {
                    m
                };
                let id = next_mid().to_string();
                let mut_file = mut_dir.to_str().unwrap().to_owned() + &id + DOT_SOL;
                let mut_path = Path::new(&mut_file);

                log::info!(
                    "Found a valid mutant of type {}",
                    ansi_term::Colour::Cyan.paint(mtype.to_string()),
                );

                std::fs::write(mut_path, &m)?;
                log::info!(
                    "{}: Mutant written at {:?}",
                    ansi_term::Colour::Green.paint("SUCCESS"),
                    mut_path
                );
                let diff = Self::diff_mutant(orig_path, mut_path)?;
                let mut_json = serde_json::json!({
                "name" : &mut_file,
                "description" : mtype.to_string(),
                "id" : &id,
                "diff": &diff,
                });
                mutants.push((mut_path.to_owned(), mut_json));
            }
        }
        Ok(mutants)
    }

    /// Logs the diff of the mutants w.r.t. the origin program.
    fn diff_mutant(orig: &Path, mutant: &Path) -> Result<String, Box<dyn Error>> {
        let (succ, diff, _) = invoke_command(
            "diff",
            vec![
                orig.to_str().unwrap(),
                mutant.to_str().unwrap(),
                // "--color=always",
            ],
        )?;
        match succ.unwrap_or_else(|| panic!("diff call terminated with a signal.")) {
            0 => {
                log::info!("mutant identical to original program");
                Ok(String::new())
            }
            1 => {
                log::info!("{}", std::str::from_utf8(&diff).unwrap());
                Ok(std::str::from_utf8(&diff).unwrap().to_string())
            }
            _ => {
                log::info!("install a `diff` program to see the diff");
                Ok(String::new())
            }
        }
    }

    /// Adds a comment to indicate what kind of mutation happened.
    fn add_mutant_comment(
        src_path: &Path,
        mutant: &String,
        mut_type: &MutationType,
    ) -> Result<String, ScannerError> {
        let mut scan1 = Scanner::scan_path(src_path)?;
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
        Ok(res.concat())
    }

    /// Mutation Generator that traverses the AST and determines which points
    /// can be mutated using which mutation type,
    /// then collects all the mutations that need to be done and calls
    /// `inner_loop` where the actual mutations are done.
    pub fn get_mutations(
        self,
        is_valid: impl FnMut(&str) -> Result<bool, Box<dyn std::error::Error>>,
    ) -> Result<Vec<(PathBuf, serde_json::Value)>, Box<dyn Error>> {
        let mut_dir = self.lkup_mutant_dir()?;
        let (visitor, skip, accept) =
            Self::mk_closures(self.mutation_types, self.funcs_to_mutate, self.contract);
        // each pair represents a mutation type and the AST node on which it is applicable.
        let mutations: Vec<(MutationType, SolAST)> = self
            .mutation_root
            .traverse(visitor, skip, accept)
            .into_iter()
            .flatten()
            .collect();
        Self::inner_loop(mut_dir, self.filename, is_valid, mutations)
    }
}
