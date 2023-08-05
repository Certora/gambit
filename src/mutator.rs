use crate::{
    default_gambit_output_directory, mutation::MutationType, normalize_mutation_operator_name,
    Mutant, MutateParams, Mutation, Solc,
};
use clap::ValueEnum;
use solang::{
    file_resolver::FileResolver,
    parse_and_resolve,
    sema::{
        ast::{Expression, Namespace, Statement},
        Recurse,
    },
};
use solang_parser::pt::CodeLocation;
use std::{
    error,
    ffi::{OsStr, OsString},
    path::PathBuf,
    rc::Rc,
};

/// This module is responsible for high level logic of running mutation over
/// Solidity programs.

/// The configuration for a mutator, this specifies the details of mutation
#[derive(Debug, Clone)]
pub struct MutatorConf {
    /// Mutation operators to apply during mutation
    pub mutation_operators: Vec<MutationType>,

    /// Operators to use when an expression or statement didn't
    /// otherwise mutate. These act as a fallback to help ensure
    /// a given program point is mutated without creating too many
    /// mutants.
    pub fallback_operators: Vec<MutationType>,

    /// If this is `Some(fnames)` then only mutate functions with names in
    /// `fnames`. If this is `None` then mutate all function names
    pub funcs_to_mutate: Option<Vec<String>>,

    /// If this is `Some(c)` then only mutate SolAST `ast` when `ast.contract ==
    /// c`. When this is `None` then no constraints are given.
    pub contract: Option<String>,
}

impl From<&MutateParams> for MutatorConf {
    fn from(mutate_params: &MutateParams) -> Self {
        let mutation_operators = if let Some(ops) = &mutate_params.mutations {
            ops.iter()
                .map(|op| {
                    MutationType::from_str(normalize_mutation_operator_name(op).as_str(), true)
                        .unwrap_or_else(|_| panic!("Unrecognized mutation operator {op}"))
                })
                .collect()
        } else {
            MutationType::default_mutation_operators()
        };
        let fallback_operators = if let Some(ops) = &mutate_params.fallback_mutations {
            ops.iter()
                .map(|op| {
                    MutationType::from_str(normalize_mutation_operator_name(op).as_str(), true)
                        .unwrap_or_else(|_| panic!("Unrecognized mutation operator {op}"))
                })
                .collect()
        } else {
            MutationType::default_fallback_mutation_operators()
        };
        MutatorConf {
            mutation_operators,
            fallback_operators,
            funcs_to_mutate: mutate_params.functions.clone(),
            contract: mutate_params.contract.clone(),
        }
    }
}

/// The mutator performs the actual logic of mutating a program, writes
pub struct Mutator {
    /// Configuration for this mutator
    pub conf: MutatorConf,

    /// The original sources
    pub filenames: Vec<String>,

    /// The mutants, in order of generation
    pub mutants: Vec<Mutant>,

    /// The file resolver
    pub file_resolver: FileResolver,

    /// The namespace being mutated
    pub namespace: Option<Rc<Namespace>>,

    /// A temporary directory to store intermediate work
    _tmp: PathBuf,
}

impl std::fmt::Debug for Mutator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Mutator")
            .field("conf", &self.conf)
            .field("file_names", &self.filenames)
            .field("mutants", &self.mutants)
            .field("_tmp", &self._tmp)
            .finish()
    }
}

impl From<&MutateParams> for Mutator {
    fn from(value: &MutateParams) -> Self {
        let conf = MutatorConf::from(value);
        let mut solc = Solc::new(
            value.solc.clone(),
            value
                .outdir
                .clone()
                .unwrap_or(default_gambit_output_directory())
                .into(),
        );
        solc.with_optimize(value.solc_optimize);

        if let Some(basepath) = value.solc_base_path.clone() {
            solc.with_basepath(basepath);
        }
        if let Some(allowpaths) = value.solc_allow_paths.clone() {
            solc.with_allow_paths(allowpaths);
        }
        if let Some(remappings) = value.solc_remappings.clone() {
            solc.with_remappings(remappings);
        }

        let mut filenames: Vec<String> = vec![];
        if let Some(filename) = &value.filename {
            log::info!("Creating Source from filename: {}", filename);
            filenames.push(filename.clone());
        }

        let mut file_resolver = FileResolver::new();

        // Add base path to file resolver
        if value.import_paths.is_empty() {
            file_resolver
                .add_import_path(&PathBuf::from("."))
                .expect(format!("Failed to add import path {}", ".").as_str());
        } else {
            for import_path in value.import_paths.iter() {
                file_resolver
                    .add_import_path(&PathBuf::from(import_path))
                    .expect(format!("Failed to add import path {}", import_path).as_str());
            }
        }

        // Add any remappings to file resolver
        if let Some(remappings) = &value.solc_remappings {
            for rm in remappings {
                let split_rm: Vec<&str> = rm.split("=").collect();
                if split_rm.len() != 2 {
                    panic!("Invalid remapping: {}", rm);
                }
                file_resolver
                    .add_import_map(OsString::from(split_rm[0]), PathBuf::from(split_rm[1]))
                    .unwrap();
            }
        }

        if let Some(allow_paths) = &value.solc_allow_paths {
            for allow_path in allow_paths.iter() {
                file_resolver
                    .add_import_path(&PathBuf::from(allow_path))
                    .expect(
                        format!("Failed to add allow_path as import path: {}", allow_path).as_str(),
                    )
            }
        }

        let mutator = Mutator::new(conf, file_resolver, filenames, solc);
        mutator
    }
}

impl Mutator {
    pub fn new(
        conf: MutatorConf,
        file_resolver: FileResolver,
        filenames: Vec<String>,
        solc: Solc,
    ) -> Mutator {
        log::info!(
            "Creating mutator:\n   conf: {:#?}\n    sources: {:?}\n    solc: {:#?}",
            conf,
            filenames,
            solc
        );
        Mutator {
            conf,
            filenames,
            mutants: vec![],
            file_resolver,
            namespace: None,
            _tmp: "".into(),
        }
    }

    pub fn mutation_operators(&self) -> &[MutationType] {
        &self.conf.mutation_operators.as_slice()
    }

    pub fn fallback_mutation_operators(&self) -> &[MutationType] {
        &self.conf.fallback_operators.as_slice()
    }

    /// Run all mutations! This is the main external entry point into mutation.
    /// This function:
    ///
    /// 1. Mutates each file
    /// 2. TODO: Optionally validates (default: yes) all generated/filtered mutants
    ///
    /// and returns a Vec of mutants. These are not yet written to disk, and can
    /// be further validated, suppressed, and downsampled as desired.
    pub fn mutate(
        &mut self,
        filenames: Vec<String>,
    ) -> Result<&Vec<Mutant>, Box<dyn error::Error>> {
        let mut mutants: Vec<Mutant> = vec![];

        for filename in filenames.iter() {
            log::info!("Mutating file {}", filename);

            match self.mutate_file(&filename) {
                Ok(file_mutants) => {
                    log::info!("    Generated {} mutants from file", file_mutants.len());
                }
                Err(e) => {
                    log::warn!("Couldn't mutate file {}", filename);
                    log::warn!("Encountered error: {}", e);
                }
            }
        }

        self.mutants.append(&mut mutants);
        Ok(&self.mutants)
    }

    /// Mutate a single file.
    fn mutate_file(&mut self, filename: &String) -> Result<Vec<Mutant>, Box<dyn error::Error>> {
        log::info!("Parsing file {}", filename);
        let ns = Rc::new(parse_and_resolve(
            &OsStr::new(filename),
            &mut self.file_resolver,
            solang::Target::EVM,
        ));
        log::info!("Parsed namespace with:");
        log::info!("    {} files", ns.files.len());
        log::info!("    {} contracts", ns.contracts.len());
        log::info!("    {} functions", ns.functions.len());
        self.namespace = Some(ns.clone());

        let resolved = self
            .file_resolver
            .resolve_file(None, OsStr::new(filename))
            .expect(format!("Unable to resolve filename {}", filename).as_str());

        let file_path = resolved.full_path.clone();
        log::info!(
            "Resolved {} to {:?} with import path {:?}",
            filename,
            resolved,
            self.file_resolver.get_import_path(resolved.get_import_no())
        );
        // mutate functions
        for function in ns.functions.iter() {
            let start_no_mutants = self.mutants.len();
            let file = ns.files.get(function.loc.file_no());
            match file {
                Some(file) => {
                    if &file.path != &file_path {
                        continue;
                    }
                }
                None => {
                    continue;
                }
            }
            if function.has_body {
                let contract_name = if let Some(contract_no) = function.contract_no {
                    let contract = ns.contracts.get(contract_no).unwrap();
                    format!("{}::", &contract.name)
                } else {
                    "".to_string()
                };
                log::info!(
                    "Processing function body for {}{}...",
                    contract_name,
                    &function.signature
                );
                for statement in function.body.iter() {
                    statement.recurse(self, mutate_statement);
                }
                let end_no_mutants = self.mutants.len();
                log::info!(
                    "    ...generated {} mutants",
                    end_no_mutants - start_no_mutants
                );
            }
        }
        self.namespace = None;
        Ok(self.mutants.clone())
    }

    /// Get a slice of the mutants produced by this mutator
    pub fn mutants(&self) -> &[Mutant] {
        &self.mutants
    }

    pub fn filenames(&self) -> &Vec<String> {
        &self.filenames
    }

    pub fn apply_operators_to_expression(&mut self, expr: &Expression) {
        if let Some(_) = expr.loc().try_file_no() {
            let mut mutants = vec![];
            for op in self.mutation_operators() {
                if op.is_fallback_mutation(self) {
                    continue;
                }
                mutants.append(&mut op.mutate_expression(self, expr));
            }
            self.mutants.append(&mut mutants);
        }
    }

    pub fn apply_fallback_operators_to_expression(&mut self, expr: &Expression) {
        if let Some(_) = expr.loc().try_file_no() {
            let mut mutants = vec![];
            println!("YOYOYO    ");
            for op in self.fallback_mutation_operators() {
                println!("Fallback op: {:?}", op);
                mutants.append(&mut op.mutate_expression_fallback(self, expr));
            }
            self.mutants.append(&mut mutants);
        }
    }

    pub fn apply_operators_to_statement(&mut self, stmt: &Statement) {
        if let Some(_) = stmt.loc().try_file_no() {
            let mut mutants = vec![];
            for op in self.mutation_operators() {
                mutants.append(&mut op.mutate_statement(self, stmt));
            }
            self.mutants.append(&mut mutants);
        }
    }
}

pub fn mutate_statement(statement: &Statement, mutator: &mut Mutator) -> bool {
    mutator.apply_operators_to_statement(statement);
    let num_mutants_before_expr_mutate = mutator.mutants.len();
    match statement {
        Statement::Block { .. } => true,
        Statement::VariableDecl(_, _, _, expr) => {
            match expr {
                Some(e) => {
                    e.recurse(mutator, mutate_expression);
                    if mutator.mutants.len() == num_mutants_before_expr_mutate {
                        perform_fallback_mutations(e, mutator);
                    }
                }
                None => (),
            }

            true
        }

        Statement::If(_, _, c, _, _) => {
            c.recurse(mutator, mutate_expression);
            if mutator.mutants.len() == num_mutants_before_expr_mutate {
                perform_fallback_mutations(c, mutator);
            }
            true
        }
        Statement::While(_, _, c, _) => {
            c.recurse(mutator, mutate_expression);
            println!(
                "While: {}, {}",
                num_mutants_before_expr_mutate,
                mutator.mutants.len()
            );
            if mutator.mutants.len() == num_mutants_before_expr_mutate {
                println!("HELLO");
                perform_fallback_mutations(c, mutator);
            }
            true
        }
        Statement::For { cond, .. } => {
            if let Some(cond) = cond {
                cond.recurse(mutator, mutate_expression);
                if mutator.mutants.len() == num_mutants_before_expr_mutate {
                    perform_fallback_mutations(cond, mutator);
                }
            }
            true
        }
        Statement::DoWhile(_, _, _, c) => {
            c.recurse(mutator, mutate_expression);
            if mutator.mutants.len() == num_mutants_before_expr_mutate {
                perform_fallback_mutations(c, mutator);
            }
            true
        }
        Statement::Expression(_, _, e) => {
            e.recurse(mutator, mutate_expression);
            if mutator.mutants.len() == num_mutants_before_expr_mutate {
                match e {
                    Expression::PreIncrement { .. }
                    | Expression::PreDecrement { .. }
                    | Expression::PostIncrement { .. }
                    | Expression::PostDecrement { .. } => (),

                    Expression::Assign { right, .. } => perform_fallback_mutations(right, mutator),

                    Expression::Constructor { args, .. }
                    | Expression::Builtin { args, .. }
                    | Expression::InternalFunctionCall { args, .. }
                    | Expression::ExternalFunctionCall { args, .. } => {
                        for arg in args {
                            perform_fallback_mutations(arg, mutator);
                        }
                    }
                    Expression::ExternalFunctionCallRaw { .. } => (),
                    _ => (),
                }
                perform_fallback_mutations(e, mutator);
            }
            true
        }
        Statement::Delete(_, _, e) | Statement::Destructure(_, _, e) => {
            e.recurse(mutator, mutate_expression);
            if mutator.mutants.len() == num_mutants_before_expr_mutate {
                perform_fallback_mutations(e, mutator);
            }
            true
        }
        Statement::Continue(_) => true,
        Statement::Break(_) => true,
        Statement::Return(_, rv) => {
            if let Some(rv) = rv {
                rv.recurse(mutator, mutate_expression);
                if mutator.mutants.len() == num_mutants_before_expr_mutate {
                    perform_fallback_mutations(rv, mutator);
                }
            }
            true
        }
        Statement::Revert { .. } => true,
        Statement::Emit { .. } => true,
        Statement::TryCatch(_, _, _) => true,
        Statement::Underscore(_) => false,
        Statement::Assembly(_, _) => false,
    }
}

pub fn mutate_expression(expr: &Expression, mutator: &mut Mutator) -> bool {
    mutator.apply_operators_to_expression(expr);
    true
}

pub fn perform_fallback_mutations(expr: &Expression, mutator: &mut Mutator) {
    mutator.apply_fallback_operators_to_expression(expr);
}
