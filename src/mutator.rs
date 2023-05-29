use crate::{
    default_gambit_output_directory, mutation::MutationType, source::Source, Mutant, MutateParams,
    Mutation, Solc,
};
use clap::ValueEnum;
use solang::{
    file_resolver::FileResolver,
    parse_and_resolve,
    sema::{
        ast::{Expression, Statement},
        Recurse,
    },
};
use std::{error, ffi::OsStr, path::PathBuf, rc::Rc};

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
        let mutation_operators = if let Some(ops) = &mutate_params.mutations {
            ops.iter()
                .map(|op| {
                    MutationType::from_str(op.as_str(), true)
                        .unwrap_or_else(|_| panic!("Unrecognized mutation operator {op}"))
                })
                .collect()
        } else {
            MutationType::default_mutation_operators()
        };
        MutatorConf {
            mutation_operators,
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

    /// The current source being mutated
    pub current_source: Option<Rc<Source>>,

    /// A temporary directory to store intermediate work
    _tmp: PathBuf,
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

        let sourceroot = match &value.sourceroot {
            Some(sourceroot) => PathBuf::from(sourceroot),
            None => {
                // Attempt to use CWD as the sourceroot. Ensuer that the
                // filename belongs to (is prefixed by) the sourceroot
                let sourceroot = PathBuf::from(".").canonicalize().unwrap();
                let filename = &value
                    .filename
                    .as_ref()
                    .unwrap_or_else(|| panic!("Found unresolved filename in params: {:?}", value));
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
            sources.push(Rc::new(
                Source::new(filename.into(), sourceroot)
                    .unwrap_or_else(|_| panic!("Couldn't read source {}", filename)),
            ))
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
            current_source: None,
            _tmp: "".into(),
        }
    }

    pub fn mutation_operators(&self) -> &[MutationType] {
        &self.conf.mutation_operators.as_slice()
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
        sources: Vec<Rc<Source>>,
    ) -> Result<&Vec<Mutant>, Box<dyn error::Error>> {
        let mut mutants: Vec<Mutant> = vec![];

        for source in sources.iter() {
            log::info!("Mutating source {}", source.filename().display());
            self.current_source = Some(source.clone());

            match self.mutate_file(source.clone()) {
                Ok(mut file_mutants) => {
                    log::info!("    Generated {} mutants from source", file_mutants.len());
                    mutants.append(&mut file_mutants);
                }
                Err(e) => {
                    log::warn!("Couldn't mutate source {}", source.filename().display());
                    log::warn!("Encountered error: {}", e);
                }
            }

            self.current_source = None;
        }

        self.mutants.append(&mut mutants);
        Ok(&self.mutants)
    }

    /// Mutate a single file.
    fn mutate_file(&mut self, source: Rc<Source>) -> Result<Vec<Mutant>, Box<dyn error::Error>> {
        let mut resolver = FileResolver::new();
        resolver.add_import_path(&PathBuf::from("."));
        let target = solang::Target::EVM;
        let ns = parse_and_resolve(&OsStr::new(source.filename()), &mut resolver, target);
        // mutate functions
        for function in ns.functions.iter() {
            if function.has_body {
                for statement in function.body.iter() {
                    statement.recurse(self, mutate_statement);
                }
            }
        }
        Ok(self.mutants.clone())
    }

    /// Get a slice of the mutants produced by this mutator
    pub fn mutants(&self) -> &[Mutant] {
        &self.mutants
    }

    pub fn sources(&self) -> &Vec<Rc<Source>> {
        &self.sources
    }

    pub fn apply_operators_to_expression(&mut self, expr: &Expression) {
        if let Some(source) = &self.current_source {
            let mut mutants = vec![];
            for op in self.mutation_operators() {
                mutants.append(&mut op.mutate_expression(expr, source));
            }
            self.mutants.append(&mut mutants);
        }
    }

    pub fn apply_operators_to_statement(&mut self, stmt: &Statement) {
        println!("applying ops {:?} to {:?}", self.mutation_operators(), stmt);
        if let Some(source) = &self.current_source {
            let mut mutants = vec![];
            for op in self.mutation_operators() {
                mutants.append(&mut op.mutate_statement(stmt, source));
            }
            self.mutants.append(&mut mutants);
        }
    }
}

pub fn mutate_statement(statement: &Statement, mutator: &mut Mutator) -> bool {
    mutator.apply_operators_to_statement(statement);
    match statement {
        Statement::Block { .. } => true,
        Statement::VariableDecl(_, _, _, _) => true,
        Statement::If(_, _, c, _, _) => {
            c.recurse(mutator, mutate_expression);
            true
        }
        Statement::While(_, _, c, _) => {
            c.recurse(mutator, mutate_expression);
            true
        }
        Statement::For {
            loc: _,
            reachable: _,
            init: _,
            cond,
            ..
        } => {
            if let Some(cond) = cond {
                cond.recurse(mutator, mutate_expression);
            }
            true
        }
        Statement::DoWhile(_, _, _, _) => true,
        Statement::Expression(_, _, _) => true,
        Statement::Delete(_, _, _) => true,
        Statement::Destructure(_, _, _) => true,
        Statement::Continue(_) => true,
        Statement::Break(_) => true,
        Statement::Return(_, rv) => {
            if let Some(rv) = rv {
                rv.recurse(mutator, mutate_expression)
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
    match expr {
        Expression::BoolLiteral { .. } => true,
        Expression::BytesLiteral { .. } => true,
        Expression::CodeLiteral { .. } => true,
        Expression::NumberLiteral { .. } => true,
        Expression::RationalNumberLiteral { .. } => true,
        Expression::StructLiteral { .. } => true,
        Expression::ArrayLiteral { .. } => true,
        Expression::ConstArrayLiteral { .. } => true,
        Expression::Add { .. } => true,
        Expression::Subtract { loc: _, ty, .. } => {
            println!("Type: {:?}", ty);
            true
        }
        Expression::Multiply { .. } => true,
        Expression::Divide { .. } => true,
        Expression::Modulo { .. } => true,
        Expression::Power { .. } => true,
        Expression::BitwiseOr { .. } => true,
        Expression::BitwiseAnd { .. } => true,
        Expression::BitwiseXor { .. } => true,
        Expression::ShiftLeft { .. } => true,
        Expression::ShiftRight { .. } => true,
        Expression::Variable { .. } => true,
        Expression::ConstantVariable { .. } => true,
        Expression::StorageVariable { .. } => true,
        Expression::Load { .. } => true,
        Expression::GetRef { .. } => true,
        Expression::StorageLoad { .. } => true,
        Expression::ZeroExt { .. } => true,
        Expression::SignExt { .. } => true,
        Expression::Trunc { .. } => true,
        Expression::CheckingTrunc { .. } => true,
        Expression::Cast { .. } => true,
        Expression::BytesCast { .. } => true,
        Expression::PreIncrement { .. } => true,
        Expression::PreDecrement { .. } => true,
        Expression::PostIncrement { .. } => true,
        Expression::PostDecrement { .. } => true,
        Expression::Assign { .. } => true,
        Expression::More { .. } => true,
        Expression::Less { .. } => true,
        Expression::MoreEqual { .. } => true,
        Expression::LessEqual { .. } => true,
        Expression::Equal { .. } => true,
        Expression::NotEqual { .. } => true,
        Expression::Not { .. } => true,
        Expression::BitwiseNot { .. } => true,
        Expression::Negate { .. } => true,
        Expression::ConditionalOperator { .. } => true,
        Expression::Subscript { .. } => true,
        Expression::StructMember { .. } => true,
        Expression::AllocDynamicBytes { .. } => true,
        Expression::StorageArrayLength { .. } => true,
        Expression::StringCompare { .. } => true,
        Expression::StringConcat { .. } => false,
        Expression::Or { .. } => true,
        Expression::And { .. } => true,
        Expression::InternalFunction { .. } => true,
        Expression::ExternalFunction { .. } => todo!(),
        Expression::InternalFunctionCall { .. } => todo!(),
        Expression::ExternalFunctionCall { .. } => todo!(),
        Expression::ExternalFunctionCallRaw { .. } => todo!(),
        Expression::Constructor { .. } => true,
        Expression::FormatString { .. } => true,
        Expression::Builtin { .. } => true,
        Expression::InterfaceId { .. } => true,
        Expression::List { .. } => true,
        Expression::UserDefinedOperator { .. } => true,
    }
}
