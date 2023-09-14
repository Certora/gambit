use crate::{
    default_gambit_output_directory, get_vfs_path, mutation::MutationType,
    normalize_mutation_operator_name, print_error, print_warning, Mutant, MutateParams, Mutation,
    Solc,
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
            .field("mutants", &self.mutants)
            .field("_tmp", &self._tmp)
            .finish()
    }
}

impl From<&MutateParams> for Mutator {
    fn from(params: &MutateParams) -> Self {
        let conf = MutatorConf::from(params);
        let mut solc = Solc::new(
            params.solc.clone().unwrap_or_else(|| "solc".to_string()),
            params
                .outdir
                .clone()
                .unwrap_or(default_gambit_output_directory())
                .into(),
        );
        solc.with_optimize(params.solc_optimize);

        if params.solc_base_path.is_some() {
            print_warning(
                "Invalid MutateParams: solc_base_path",
                "solc_base_path is ignored. Use import_paths instead",
            );
        }

        if params.solc_remappings.is_some() {
            print_warning(
                "Invalid MutateParams: solc_remappings",
                "solc_remappings is ignored. Use import_maps instead",
            );
        }

        if let Some(allowpaths) = params.solc_allow_paths.clone() {
            solc.with_allow_paths(allowpaths);
        }

        // Every mutator has a FileResolver. A FileResolver is a solang-provided
        // struct that resolves files, performs import resolution, and then
        // performs type resolution.
        let mut file_resolver = FileResolver::default();

        // Add import paths to file resolver
        if params.import_paths.is_empty() {
            print_error(
                "No import paths found",
                "Tried to create a Mutator without an import path",
            );
            std::process::exit(1);
        } else {
            for import_path in params.import_paths.iter() {
                file_resolver.add_import_path(&PathBuf::from(import_path));
            }
        }

        // Add any remappings to file resolver
        for rm in &params.import_maps {
            let split_rm: Vec<&str> = rm.split('=').collect();
            if split_rm.len() != 2 {
                panic!("Invalid remapping: {}", rm);
            }
            let map = split_rm[0];
            let path = split_rm[1];
            // XXX: This is a hack to deal with a Solang bug, where mapping
            // targets are canonicalized _before_ they are resolved against
            // import paths. To work around this _we_ have to resolve against
            // import paths! Rather than passing in a raw import target, we
            // will manually resolve our target against any import paths

            let target = if let Some(target) = params
                .import_paths
                .iter()
                .filter_map(|p| PathBuf::from(p).join(path).canonicalize().ok())
                .next()
            {
                target
            } else {
                print_error(
                    format!("Could not resolve remapping target {}", path).as_str(),
                    format!(
                        "Attempted to resolve {} against one of import paths [{}]",
                        path,
                        params.import_paths.join(", ")
                    )
                    .as_str(),
                );
                std::process::exit(1);
            };

            file_resolver.add_import_map(OsString::from(map), target);
        }

        if let Some(allow_paths) = &params.solc_allow_paths {
            for allow_path in allow_paths.iter() {
                file_resolver.add_import_path(&PathBuf::from(allow_path));
            }
        }

        Mutator::new(conf, file_resolver, solc)
    }
}

impl Mutator {
    pub fn new(conf: MutatorConf, file_resolver: FileResolver, solc: Solc) -> Mutator {
        log::info!(
            "Creating mutator:\n   conf: {:#?}\n    solc: {:#?}",
            conf,
            solc
        );
        Mutator {
            conf,
            mutants: vec![],
            file_resolver,
            namespace: None,
            _tmp: "".into(),
        }
    }

    pub fn mutation_operators(&self) -> &[MutationType] {
        self.conf.mutation_operators.as_slice()
    }

    pub fn fallback_mutation_operators(&self) -> &[MutationType] {
        self.conf.fallback_operators.as_slice()
    }

    /// Run all mutations! This is the main external entry point into mutation.
    /// This function mutates each file and returns a Vec of mutants. These are
    /// not yet written to disk, and can be further validated, suppressed, and
    /// downsampled as desired.
    pub fn mutate(
        &mut self,
        filenames: Vec<String>,
    ) -> Result<&Vec<Mutant>, Box<dyn error::Error>> {
        let mut mutants: Vec<Mutant> = vec![];

        for filename in filenames.iter() {
            log::info!("Mutating file {}", filename);

            match self.mutate_file(filename) {
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
        // Check if we can mutate path
        let vfs_path = get_vfs_path(&self.file_resolver, &PathBuf::from(filename));
        if vfs_path.is_none() {
            let import_paths: Vec<String> = self
                .file_resolver
                .get_import_paths()
                .iter()
                .map(|p| p.1.to_str().unwrap().to_string())
                .collect();
            print_error("File Not In Import Paths", format!("Could not mutate file {}:\nFile could not be resolved against any provided import paths.\nImport Paths: {:?}", filename, import_paths).as_str());
            std::process::exit(1);
        }
        log::info!("Parsing file {}", filename);
        let os_filename = OsStr::new(filename);
        let ns = Rc::new(parse_and_resolve(
            os_filename,
            &mut self.file_resolver,
            solang::Target::EVM,
        ));
        log::info!("Parsed namespace with:");
        log::info!("    {} files", ns.files.len());
        log::info!("    {} contracts", ns.contracts.len());
        log::info!("    {} functions", ns.functions.len());

        if ns.diagnostics.any_errors() {
            ns.print_diagnostics(&self.file_resolver, true);
            return Err("error".into());
        }

        self.namespace = Some(ns.clone());

        let resolved = match self.file_resolver.resolve_file(None, os_filename) {
            Ok(resolved) => resolved,
            Err(e) => {
                print_error(
                    format!("Unable to resolve filename {}", filename).as_str(),
                    format!("Found error {}", e).as_str(),
                );
                std::process::exit(1)
            }
        };

        let file_path = resolved.full_path.clone();
        // mutate functions
        for function in ns.functions.iter() {
            let start_no_mutants = self.mutants.len();
            let file = ns.files.get(function.loc.file_no());
            match file {
                Some(file) => {
                    if file.path != file_path {
                        continue;
                    }
                }
                None => {
                    continue;
                }
            }
            if function.is_accessor || !function.has_body {
                continue;
            }
            let contract = if let Some(contract_no) = function.contract_no {
                let contract = ns.contracts.get(contract_no).unwrap();
                Some(format!("{}", &contract.name))
            } else {
                None
            };

            let contract_name = contract.unwrap_or_else(|| "".to_string());
            let function_name = function.name.clone();
            if let Some(ref funcs_to_mutate) = self.conf.funcs_to_mutate {
                if !funcs_to_mutate.contains(&function_name) {
                    continue;
                }
            }
            if let Some(ref contract_to_mutate) = self.conf.contract {
                if &contract_name != contract_to_mutate {
                    continue;
                }
            }

            log::info!(
                "Processing function body for {}::{}...",
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
        self.namespace = None;
        Ok(self.mutants.clone())
    }

    /// Get a slice of the mutants produced by this mutator
    pub fn mutants(&self) -> &[Mutant] {
        &self.mutants
    }

    /// Apply all regular mutation operators to an expression, and add those
    /// mutants to self.mutants. Return the slice of new mutants
    pub fn apply_operators_to_expression(&mut self, expr: &Expression) -> &[Mutant] {
        let num_mutants_at_start = self.mutants.len();
        if expr.loc().try_file_no().is_some() {
            let mut mutants = vec![];
            for op in self.mutation_operators() {
                if op.is_fallback_mutation(self) {
                    continue;
                }
                mutants.append(&mut op.mutate_expression(self, expr));
            }
            self.mutants.append(&mut mutants);
        }
        return &self.mutants[num_mutants_at_start..self.mutants.len()];
    }

    /// Apply all fallback mutation operators to an expression, and add those
    /// mutants to self.mutants. Return the slice of new mutants
    pub fn apply_fallback_operators_to_expression(&mut self, expr: &Expression) -> &[Mutant] {
        let num_mutants_at_start = self.mutants.len();
        if expr.loc().try_file_no().is_some() {
            let mut mutants = vec![];
            for op in self.fallback_mutation_operators() {
                mutants.append(&mut op.mutate_expression_fallback(self, expr));
            }
            self.mutants.append(&mut mutants);
        }
        return &self.mutants[num_mutants_at_start..self.mutants.len()];
    }

    /// Apply all regular mutation operators to a statement, and add those
    /// mutants to self.mutants. Return the slice of new mutants
    pub fn apply_operators_to_statement(&mut self, stmt: &Statement) -> &[Mutant] {
        let num_mutants_at_start = self.mutants.len();
        if stmt.loc().try_file_no().is_some() {
            let mut mutants = vec![];
            for op in self.mutation_operators() {
                mutants.append(&mut op.mutate_statement(self, stmt));
            }
            self.mutants.append(&mut mutants);
        }
        return &self.mutants[num_mutants_at_start..self.mutants.len()];
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
            if mutator.mutants.len() == num_mutants_before_expr_mutate {
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
