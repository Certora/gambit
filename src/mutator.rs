/// This module is responsible for high level logic of running mutation over
/// Solidity programs.
use crate::{
    mutation::MutationType, source::Source, Mutant, MutateParams, Mutation, SolAST, SolASTVisitor,
    Solc,
};
use std::{error, path::PathBuf, rc::Rc};

/// The configuration for a mutator, this details how a given mutation run
/// should operate
#[derive(Debug, Clone)]
pub struct MutatorConf {
    /// Mutation operators to apply during mutation
    mutation_operators: Vec<MutationType>,

    /// If this is `Some(fnames)` then only mutate functions with names in
    /// `fnames`. If this is `None` then mutate all function names
    pub funcs_to_mutate: Option<Vec<String>>,

    /// If this is `Some(c)` then only mutate SolAST `ast` when `ast.contract ==
    /// c`. When this is `None` then no constraints are given.
    pub contract: Option<String>,
}
/// The mutator performs the actual logic of mutating a program, writes
#[derive(Debug)]
pub struct Mutator {
    /// Configuration for this mutator
    pub conf: MutatorConf,

    /// The original sources
    pub sources: Vec<Rc<Source>>,

    /// The mutants, in order of generation
    pub mutants: Vec<Mutant>,

    /// Solc configuration
    solc: Solc,

    /// A temporary directory to store intermediate work
    _tmp: PathBuf,
}

impl From<&MutateParams> for Mutator {
    fn from(value: &MutateParams) -> Self {
        let conf = MutatorConf::from(value);
        let solc = Solc::new(value.solc.clone(), value.outdir.clone().into());
        let mut sources: Vec<Rc<Source>> = vec![];
        if let Some(fns) = &value.filename {
            fns.iter().for_each(|f| {
                sources.push(Rc::new(
                    Source::new(f.into()).expect("Couldn't read source"),
                ))
            });
        }
        Mutator {
            conf,
            sources: sources,
            mutants: vec![],
            solc,
            _tmp: "".into(),
        }
    }
}

impl Mutator {
    /// Run all mutations! This is the main external entry point into mutation. This
    /// 1. TODO: Mutates each file
    /// 2. TODO: Optionally downsamples (default: no) the mutants if a Filter is provided
    /// 3. TODO: Optionally validates (default: yes) all generated/filtered mutants
    /// 4. TODO: Writes mutants to disk
    ///    1. TODO: Writes the mutant log
    ///    2. TODO: Writes each mutant's diff
    ///    3. TODO: Optionally writes each mutant's source (default: false)
    pub fn mutate(&mut self) -> Result<&Vec<Mutant>, Box<dyn error::Error>> {
        let mut mutants: Vec<Mutant> = vec![];

        for source in self.sources.iter() {
            let solc = &self.solc;
            let mut file_mutants = self.mutate_file(source.clone(), solc)?;
            mutants.append(&mut file_mutants);
        }

        self.mutants.append(&mut mutants);
        Ok(&self.mutants)
    }

    /// Mutate a single file.
    fn mutate_file(
        &self,
        source: Rc<Source>,
        solc: &Solc,
    ) -> Result<Vec<Mutant>, Box<dyn error::Error>> {
        let ast = solc.compile(source.filename())?;
        Ok(ast.traverse(self, source).into_iter().flatten().collect())
    }

    /// Check if a node in the AST is an assert.
    pub fn is_assert_call(node: &SolAST) -> bool {
        node.name().map_or_else(|| false, |n| n == "assert")
    }
}

impl From<&MutateParams> for MutatorConf {
    fn from(mutate_params: &MutateParams) -> Self {
        MutatorConf {
            mutation_operators: MutationType::default_mutation_operators(),
            funcs_to_mutate: None,
            contract: None,
        }
    }
}

impl SolASTVisitor<Rc<Source>, Vec<Mutant>> for Mutator {
    fn skip_node(&self, node: &SolAST, _source: &Rc<Source>) -> bool {
        if let Some(e) = &node.element {
            if let Some(e) = e.as_object() {
                if e.contains_key("contractKind") {
                    let contract_name = e.get("name".into()).unwrap();
                    if let Some(contract) = &self.conf.contract {
                        return contract != &contract_name.to_string();
                    }
                }
            }
        }
        false
    }

    fn visit_node(&self, node: &SolAST, arg: &Rc<Source>) -> Option<Vec<Mutant>> {
        let op_node_pairs: Vec<Mutant> = self
            .conf
            .mutation_operators
            .iter()
            .filter(|m| m.applies_to(node))
            .map(|m| m.mutate(node, arg.clone()))
            .flatten()
            .into_iter()
            .collect();

        Some(op_node_pairs)
    }
}
