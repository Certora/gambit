use crate::{
    default_gambit_output_directory, mutation::MutationType, source::Source, Mutant, MutateParams,
    Solc,
};
use clap::ValueEnum;
use solang_parser;
use solang_parser::pt::{
    ContractDefinition, Expression, FunctionDefinition, SourceUnit, SourceUnitPart, Statement,
};
use std::{error, path::PathBuf, rc::Rc};

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
    pub fn mutate(&mut self) -> Result<&Vec<Mutant>, Box<dyn error::Error>> {
        let mut mutants: Vec<Mutant> = vec![];

        for source in self.sources.iter() {
            log::info!("Mutating source {}", source.filename().display());

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
        }

        self.mutants.append(&mut mutants);
        Ok(&self.mutants)
    }

    /// Mutate a single file.
    fn mutate_file(&self, source: Rc<Source>) -> Result<Vec<Mutant>, Box<dyn error::Error>> {
        let (pt, comments) = solang_parser::parse(&source.filename_as_str(), 0).unwrap();
        let result = mutate_source_unit(&pt, &self.mutation_operators())?;
        Ok(result)
    }

    /// Get a slice of the mutants produced by this mutator
    pub fn mutants(&self) -> &[Mutant] {
        &self.mutants
    }

    pub fn sources(&self) -> &Vec<Rc<Source>> {
        &self.sources
    }
}

pub fn mutate_source_unit(
    source_unit: &SourceUnit,
    mut_ops: &[MutationType],
) -> Result<Vec<Mutant>, Box<dyn error::Error>> {
    let mut mutants: Vec<Mutant> = Vec::default();
    for part in source_unit.0.iter() {
        mutants.append(&mut mutate_source_unit_part(part, mut_ops)?);
    }
    Ok(mutants)
}

pub fn mutate_source_unit_part(
    part: &SourceUnitPart,
    mut_ops: &[MutationType],
) -> Result<Vec<Mutant>, Box<dyn error::Error>> {
    match part {
        SourceUnitPart::ContractDefinition(cd) => mutate_contract_definition(cd.as_ref(), mut_ops),
        SourceUnitPart::FunctionDefinition(fd) => todo!(),
        SourceUnitPart::VariableDefinition(_) => todo!(),
        _ => Ok(vec![]),
    }
}
pub fn mutate_contract_definition(
    contract_definition: &ContractDefinition,
    mut_ops: &[MutationType],
) -> Result<Vec<Mutant>, Box<dyn error::Error>> {
    let mut mutants: Vec<Mutant> = Vec::default();
    for part in contract_definition.parts.iter() {
        todo!()
    }
    Ok(mutants)
}

pub fn mutate_function_definition(
    function_definition: &FunctionDefinition,
    mut_ops: &Vec<MutationType>,
) -> Result<Vec<Mutant>, Box<dyn error::Error>> {
    if let Some(statement) = &function_definition.body {
        mutate_statement(statement, mut_ops)
    } else {
        Ok(vec![])
    }
}

pub fn mutate_statement(
    statement: &Statement,
    mut_ops: &Vec<MutationType>,
) -> Result<Vec<Mutant>, Box<dyn error::Error>> {
    match statement {
        Statement::Block {
            loc,
            unchecked,
            statements,
        } => {
            let mut mutants: Vec<Mutant> = Vec::default();
            for statement in statements.iter() {
                mutants.append(&mut mutate_statement(statement, mut_ops)?);
            }
            Ok(mutants)
        }
        Statement::Assembly {
            loc,
            dialect,
            flags,
            block,
        } => todo!(),
        Statement::If(_, cond, then, els) => {
            todo!()
        }
        Statement::While(_, cond, body) => {
            todo!()
        }
        Statement::Expression(_, expr) => {
            todo!()
        }
        Statement::VariableDefinition(_, var_decl, initializer) => {
            todo!()
        }
        Statement::For(_, init, cond, update, body) => {
            todo!()
        }
        Statement::DoWhile(_, body, cond) => {
            todo!()
        }
        Statement::Continue(_) => todo!(),
        Statement::Break(_) => todo!(),
        Statement::Return(_, expr) => {
            todo!()
        }
        Statement::Revert(_, _, _) => todo!(),
        Statement::RevertNamedArgs(_, _, _) => todo!(),
        Statement::Emit(_, _) => todo!(),
        Statement::Try(_, _, _, _) => todo!(),
        _ => Ok(vec![]),
    }
}

pub fn mutate_expression(
    expr: &Expression,
    mut_ops: &Vec<MutationType>,
) -> Result<Vec<Mutant>, Box<dyn error::Error>> {
    match expr {
        Expression::PostIncrement(_, _) => todo!(),
        Expression::PostDecrement(_, _) => todo!(),
        Expression::New(_, _) => todo!(),
        Expression::ArraySubscript(_, _, _) => todo!(),
        Expression::ArraySlice(_, _, _, _) => todo!(),
        Expression::Parenthesis(_, _) => todo!(),
        Expression::MemberAccess(_, _, _) => todo!(),
        Expression::FunctionCall(_, func, args) => todo!(),
        Expression::FunctionCallBlock(_, _, _) => todo!(),
        Expression::NamedFunctionCall(_, _, _) => todo!(),
        Expression::Not(_, _) => todo!(),
        Expression::BitwiseNot(_, _) => todo!(),
        Expression::Delete(_, _) => todo!(),
        Expression::PreIncrement(_, _) => todo!(),
        Expression::PreDecrement(_, _) => todo!(),
        Expression::UnaryPlus(_, _) => todo!(),
        Expression::Negate(_, _) => todo!(),
        Expression::Power(_, _, _) => todo!(),
        Expression::Multiply(_, _, _) => todo!(),
        Expression::Divide(_, _, _) => todo!(),
        Expression::Modulo(_, _, _) => todo!(),
        Expression::Add(_, _, _) => todo!(),
        Expression::Subtract(_, _, _) => todo!(),
        Expression::ShiftLeft(_, _, _) => todo!(),
        Expression::ShiftRight(_, _, _) => todo!(),
        Expression::BitwiseAnd(_, _, _) => todo!(),
        Expression::BitwiseXor(_, _, _) => todo!(),
        Expression::BitwiseOr(_, _, _) => todo!(),
        Expression::Less(_, _, _) => todo!(),
        Expression::More(_, _, _) => todo!(),
        Expression::LessEqual(_, _, _) => todo!(),
        Expression::MoreEqual(_, _, _) => todo!(),
        Expression::Equal(_, _, _) => todo!(),
        Expression::NotEqual(_, _, _) => todo!(),
        Expression::And(_, _, _) => todo!(),
        Expression::Or(_, _, _) => todo!(),
        Expression::ConditionalOperator(_, _, _, _) => todo!(),
        Expression::Assign(_, _, _) => todo!(),
        Expression::AssignOr(_, _, _) => todo!(),
        Expression::AssignAnd(_, _, _) => todo!(),
        Expression::AssignXor(_, _, _) => todo!(),
        Expression::AssignShiftLeft(_, _, _) => todo!(),
        Expression::AssignShiftRight(_, _, _) => todo!(),
        Expression::AssignAdd(_, _, _) => todo!(),
        Expression::AssignSubtract(_, _, _) => todo!(),
        Expression::AssignMultiply(_, _, _) => todo!(),
        Expression::AssignDivide(_, _, _) => todo!(),
        Expression::AssignModulo(_, _, _) => todo!(),
        Expression::BoolLiteral(_, _) => todo!(),
        Expression::NumberLiteral(_, _, _, _) => todo!(),
        Expression::RationalNumberLiteral(_, _, _, _, _) => todo!(),
        Expression::HexNumberLiteral(_, _, _) => todo!(),
        Expression::StringLiteral(_) => todo!(),
        Expression::Type(_, _) => todo!(),
        Expression::HexLiteral(_) => todo!(),
        Expression::AddressLiteral(_, _) => todo!(),
        Expression::Variable(_) => todo!(),
        Expression::List(_, _) => todo!(),
        Expression::ArrayLiteral(_, _) => todo!(),
        Expression::This(_) => todo!(),
    }
}
