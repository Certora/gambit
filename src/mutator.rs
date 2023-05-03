use crate::{
    mutation::MutationType, source::Source, Mutant, MutantWriter, MutateParams, Mutation, SolAST,
    SolASTVisitor, Solc,
};
use std::{error, path::PathBuf, rc::Rc};
use tempfile::{tempdir, NamedTempFile};

/// This module is responsible for high level logic of running mutation over
/// Solidity programs.

/// The configuration for a mutator, this specifies the details of mutation
#[derive(Debug, Clone)]
pub struct MutatorConf {
    /// Mutation operators to apply during mutation
    pub mutation_operators: Vec<MutationType>,

    /// If this is `Some(fnames)` then only mutate functions with names in
    /// `fnames`. If this is `None` then mutate all function names
    pub funcs_to_mutate: Option<Vec<String>>,

    /// If this is `Some(c)` then only mutate SolAST `ast` when `ast.contract ==
    /// c`. When this is `None` then no constraints are given.
    pub contract: Option<String>,
}

impl From<&MutateParams> for MutatorConf {
    fn from(mutate_params: &MutateParams) -> Self {
        MutatorConf {
            mutation_operators: MutationType::default_mutation_operators(),
            funcs_to_mutate: mutate_params.functions.clone(),
            contract: mutate_params.contract.clone(),
        }
    }
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
        let mut solc = Solc::new(value.solc.clone(), value.outdir.clone().into());
        solc.with_optimize(value.solc_optimize);
        if let Some(basepath) = value.solc_basepath.clone() {
            solc.with_basepath(basepath);
        }
        if let Some(allowpaths) = value.solc_allowpaths.clone() {
            solc.with_allow_paths(allowpaths);
        }
        if let Some(remappings) = value.solc_remappings.clone() {
            solc.with_remappings(remappings);
        }

        let sourceroot = match &value.sourceroot {
            Some(sourceroot) => PathBuf::from(sourceroot),
            None => {
                // Attempt to use CWD as the sourceroot. Ensuer that the
                // filename belongs to (is prefixed by) the sourceroot
                let sourceroot = PathBuf::from(".").canonicalize().unwrap();
                let filename = &value
                    .filename
                    .as_ref()
                    .expect(format!("Found unresolved filename in params: {:?}", value).as_str());
                let filepath = PathBuf::from(filename).canonicalize().unwrap();
                if !&filepath.starts_with(&sourceroot) {
                    panic!("Unresolved sourceroot! Attempted to use the current working directory {} but filename {} was not a descendent.", sourceroot.display(), filepath.display());
                }

                sourceroot
            }
        };

        let mut sources: Vec<Rc<Source>> = vec![];
        if let Some(filename) = &value.filename {
            log::info!("Creating Source from filename: {}", filename);
            sources
                .push(Rc::new(Source::new(filename.into(), sourceroot).expect(
                    format!("Couldn't read source {}", filename).as_str(),
                )))
        }
        Mutator::new(conf, sources, solc)
    }
}

impl Mutator {
    pub fn new(conf: MutatorConf, sources: Vec<Rc<Source>>, solc: Solc) -> Mutator {
        log::info!(
            "Creating mutator:\n   conf: {:#?}\n    sources: {:?}\n    solc: {:#?}",
            conf,
            sources,
            solc
        );
        Mutator {
            conf,
            sources,
            mutants: vec![],
            solc,
            _tmp: "".into(),
        }
    }

    /// Run all mutations! This is the main external entry point into mutation.
    /// This function:
    ///
    /// 1. Mutates each file
    /// 2. TODO: Optionally validates (default: yes) all generated/filtered mutants
    ///
    /// and returns a Vec of mutants. These are not yet written to disk, and can
    /// be further validated, suppressed, and downsampled as desired.
    pub fn mutate(&mut self) -> Result<&Vec<Mutant>, Box<dyn error::Error>> {
        let mut mutants: Vec<Mutant> = vec![];

        let solc = &self.solc;
        for source in self.sources.iter() {
            log::info!("Mutating source {}", source.filename().display());

            match self.mutate_file(source.clone(), solc) {
                Ok(mut file_mutants) => {
                    log::info!("    Generated {} mutants from source", file_mutants.len());
                    mutants.append(&mut file_mutants);
                }
                Err(e) => {
                    log::warn!("Couldn't mutate source {}", source.filename().display());
                    log::warn!("Encountered error: {}", e);
                }
            }
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
        let ast = solc.compile_ast(source.filename())?;
        if !solc.output_directory().exists() {
            log::debug!(
                "[Pre traverse] Output directory {} doesn't exist!",
                solc.output_directory().display()
            );
        }
        let result = ast.traverse(self, source).into_iter().flatten().collect();
        if !solc.output_directory().exists() {
            log::debug!(
                "[Post traverse] Output directory {} doesn't exist!",
                solc.output_directory().display()
            );
        }
        Ok(result)
    }

    /// Check if a node in the AST is an assert.
    pub fn is_assert_call(node: &SolAST) -> bool {
        node.name().map_or_else(|| false, |n| n == "assert")
    }

    /// Get a slice of the mutants produced by this mutator
    pub fn mutants(&self) -> &[Mutant] {
        &self.mutants
    }

    pub fn sources(&self) -> &Vec<Rc<Source>> {
        &self.sources
    }

    pub fn solc(&self) -> &Solc {
        &self.solc
    }

    /// validate a mutant by writing it to disk and compiling it. If compilation
    /// fails then this is an invalid mutant.
    pub fn validate_mutant(&self, mutant: &Mutant) -> Result<bool, Box<dyn error::Error>> {
        let source_filename = mutant.source.filename();
        let source_parent_dir = source_filename.parent().unwrap();
        let mutant_file = NamedTempFile::new_in(source_parent_dir)?;
        let mutant_file_path = mutant_file.path();
        log::debug!(
            "Validating mutant of {}: copying mutated code to {}",
            source_filename.display(),
            mutant_file_path.display()
        );
        let dir = tempdir()?;
        MutantWriter::write_mutant_to_file(mutant_file_path, &mutant)?;
        let code = match self.solc().compile(mutant_file_path, dir.path()) {
            Ok((code, _, _)) => code == 0,
            Err(_) => false,
        };
        Ok(code)
    }

    pub fn get_valid_mutants(&self, mutants: &Vec<Mutant>) -> Vec<Mutant> {
        log::info!("Validating mutants...");
        let mut valid_mutants = vec![];
        for m in mutants.iter() {
            match self.validate_mutant(m) {
                Ok(true) => valid_mutants.push(m.clone()),
                _ => (),
            }
        }
        valid_mutants
    }
}

impl SolASTVisitor<Rc<Source>, Vec<Mutant>> for Mutator {
    fn skip_node(&self, node: &SolAST, _source: &Rc<Source>) -> bool {
        if let Some(e) = &node.element {
            if let Some(e_obj) = e.as_object() {
                if e_obj.contains_key("contractKind") {
                    let contract_name = e_obj.get("name".into()).unwrap();
                    if let Some(contract) = &self.conf.contract {
                        return contract != &contract_name.as_str().unwrap();
                    } else {
                        return false;
                    }
                } else if node.node_kind() == Some("function".to_string()) {
                    match &self.conf.funcs_to_mutate {
                        Some(fns) => {
                            if let Some(name) = node.name() {
                                return !fns.contains(&name);
                            }
                            return true;
                        }
                        None => {
                            return false;
                        }
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
