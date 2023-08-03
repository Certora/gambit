use crate::{get_import_path, get_indent, Mutator};
use clap::ValueEnum;
use num_bigint::BigInt;
use num_traits::{One, Zero};
use serde::{Deserialize, Serialize};
use solang::{
    file_resolver::FileResolver,
    sema::ast::{CallTy, Expression, Namespace, RetrieveType, Statement, Type},
};
use solang_parser::pt::{CodeLocation, Loc};
use std::{
    error,
    fmt::{Debug, Display},
    path::PathBuf,
    rc::Rc,
    sync::Arc,
};

/// MutantLoc describes all location-based data of a mutant, including which
/// file the mutant mutates, the original solang Loc, and the line and column
/// numbers
#[derive(Clone)]
pub struct MutantLoc {
    /// The location of the node that is mutated
    pub loc: Loc,
    /// The (starting) line number of the mode being mutated
    pub line_no: usize,
    /// The column number of the node being mutated
    pub col_no: usize,
    /// The full path to the original source file
    pub path: PathBuf,
    /// The solidity path, relative to its import root, to the original source
    /// file; if a file path is specified absolutely then this is None
    pub sol_path: Option<PathBuf>,
}

impl Debug for MutantLoc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MutantLoc")
            .field("path", &self.path.display())
            .field("loc", &self.loc)
            .field("line_no", &self.line_no)
            .field("col_no", &self.col_no)
            .finish()
    }
}

impl MutantLoc {
    pub fn new(loc: Loc, resolver: &FileResolver, namespace: Rc<Namespace>) -> MutantLoc {
        let file = namespace.files.get(loc.file_no()).unwrap();
        let (_, line_no, col_no, _) = resolver.get_line_and_offset_from_loc(file, &loc);
        let path = file.path.clone();
        let import_path = get_import_path(
            resolver,
            file.import_no
                .expect("Expected an import no but found None"),
        )
        .expect("Expected an import path but found None");
        let sol_path = path
            .strip_prefix(&import_path)
            .expect(
                format!(
                    "Could not strip prefix {:?} from path {:?}",
                    &import_path, &path
                )
                .as_str(),
            )
            .to_path_buf();

        MutantLoc {
            loc,
            line_no,
            col_no,
            path,
            sol_path: Some(sol_path),
        }
    }
}

/// This struct describes a mutant.
#[derive(Clone)]
pub struct Mutant {
    /// The location of the mutant (including file number, start, and end)
    pub mutant_loc: MutantLoc,

    /// The mutation operator that was applied to generate this mutant
    pub op: MutationType,

    /// Original file's source
    pub source: Arc<str>,

    /// Original text to be replaced
    pub orig: String,

    /// The string replacement
    pub repl: String,
}

impl Mutant {
    pub fn new(
        resolver: &FileResolver,
        namespace: Rc<Namespace>,
        loc: Loc,
        op: MutationType,
        orig: String,
        repl: String,
    ) -> Mutant {
        if let Loc::File(file_no, _, _) = loc {
            let source = resolver.get_contents_of_file_no(file_no).unwrap();
            let mutant_loc = MutantLoc::new(loc, resolver, namespace);
            Mutant {
                mutant_loc,
                op,
                source,
                orig,
                repl,
            }
        } else {
            panic!("Location must be Loc::File(...), but found {:?}", loc)
        }
    }

    pub fn loc(&self) -> &Loc {
        &self.mutant_loc.loc
    }

    pub fn path(&self) -> &PathBuf {
        &self.mutant_loc.path
    }

    pub fn sol_path(&self) -> Option<&PathBuf> {
        self.mutant_loc.sol_path.as_ref()
    }

    pub fn get_line_column(&self) -> (usize, usize) {
        let mloc = &self.mutant_loc;
        (mloc.line_no, mloc.col_no)
    }

    /// Render this mutant as String with the full source file contents
    ///
    /// TODO: Cache these contents: this data might be needed multiple times,
    /// and if so this should be cached as it currently involves file IO (though
    /// Source::contents() should also be cached)
    pub fn mutant_source(&self) -> Result<String, Box<dyn error::Error>> {
        let loc = self.loc();
        let start = loc.start();
        let end = loc.end();

        let contents: Arc<str> = self.source.clone();
        let orig = &contents[start..end];

        let prelude = &contents[0..start];
        let postlude = &contents[end..contents.len()];

        let (line_no, _) = self.get_line_column();

        let res = [prelude, self.repl.as_str(), postlude].concat();
        let mut_string = res;
        let mut lines = mut_string.lines();

        let mut lines2 = vec![];
        for _ in 1..line_no {
            lines2.push(lines.next().unwrap());
        }

        let mut_line = lines.next().unwrap();
        let orig_line = contents.lines().nth(line_no).unwrap();

        let indent = get_indent(mut_line);
        let comment = format!(
            "{}/// {}(`{}` |==> `{}`) of: `{}`",
            indent,
            self.op.to_string(),
            orig.trim(),
            self.repl,
            orig_line.trim()
        );
        lines2.push(&comment);
        lines2.push(mut_line);

        for line in lines {
            lines2.push(line);
        }

        // XXX: this is a hack to avoid trailing newline diffs
        if contents.chars().last().unwrap() == '\n' {
            lines2.push("");
        }
        Ok(lines2.join("\n"))
    }

    pub fn original_source(&self) -> Arc<str> {
        self.source.clone()
    }
}

impl Display for Mutant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: {} |==> {}",
            self.op.to_string(),
            &self.orig,
            &self.repl
        )
    }
}

impl Debug for Mutant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Mutant")
            .field("op", &self.op)
            .field("orig", &self.orig)
            .field("repl", &self.repl)
            .field("mutant_loc", &self.mutant_loc)
            .finish()
    }
}

/// Every kind of mutation implements this trait. A mutation can check if it
/// applies to an AST node, and can mutate an AST node.
pub trait Mutation {
    fn mutate_statement(&self, mutator: &Mutator, stmt: &Statement) -> Vec<Mutant>;

    fn mutate_expression(&self, mutator: &Mutator, expr: &Expression) -> Vec<Mutant>;
}

/// Kinds of mutations.
#[derive(Hash, Eq, PartialEq, Clone, Copy, Debug, ValueEnum, Deserialize, Serialize)]
pub enum MutationType {
    ArithmeticOperatorReplacement,
    BitwiseOperatorReplacement,
    ElimDelegateCall,
    LiteralValueReplacement,
    LogicalOperatorReplacement,
    RelationalOperatorReplacement,
    ShiftOperatorReplacement,
    StatementDeletion,
    UnaryOperatorReplacement,
    ExpressionValueReplacement,
}

impl ToString for MutationType {
    fn to_string(&self) -> String {
        let str = match self {
            MutationType::LiteralValueReplacement => "LiteralValueReplacement",
            MutationType::BitwiseOperatorReplacement => "ConditionalOperatorReplacement",
            MutationType::RelationalOperatorReplacement => "RelationalOperatorReplacement",
            MutationType::ArithmeticOperatorReplacement => "ArithmeticOperatorReplacement",
            MutationType::LogicalOperatorReplacement => "LogicalOperatorReplacement",
            MutationType::ShiftOperatorReplacement => "ShiftOperatorReplacement",
            MutationType::UnaryOperatorReplacement => "UnaryOperatorReplacement",
            MutationType::ExpressionValueReplacement => "ExpressionOperatorReplacement",
            MutationType::StatementDeletion => "StatementDeletion",

            MutationType::ElimDelegateCall => "ElimDelegateCall",
        };
        str.into()
    }
}

impl Mutation for MutationType {
    fn mutate_statement(&self, mutator: &Mutator, stmt: &Statement) -> Vec<Mutant> {
        let file_no = stmt.loc().file_no();
        let resolver = &mutator.file_resolver;
        let ns = mutator
            .namespace
            .as_ref()
            .expect("Cannot mutate an expression without a set namespace")
            .clone();
        let contents = resolver.get_contents_of_file_no(file_no).unwrap();
        let loc = stmt.loc();
        if let None = loc.try_file_no() {
            println!("No file");
            return vec![];
        }
        match self {
            MutationType::StatementDeletion => {
                statement_deletion(self, resolver, ns, stmt, &contents)
            }
            _ => vec![],
        }
    }

    fn mutate_expression(&self, mutator: &Mutator, expr: &Expression) -> Vec<Mutant> {
        let file_no = expr.loc().file_no();
        let resolver = &mutator.file_resolver;
        let contents = &resolver.get_contents_of_file_no(file_no).unwrap();
        let ns = mutator
            .namespace
            .as_ref()
            .expect("Cannot mutate an expression without a set namespace")
            .clone();
        match self {
            // Binary Operators
            MutationType::ArithmeticOperatorReplacement => {
                arith_op_replacement(self, resolver, ns, expr, contents)
            }
            MutationType::ShiftOperatorReplacement => {
                shift_op_replacement(self, resolver, ns, expr, contents)
            }
            MutationType::BitwiseOperatorReplacement => {
                bitwise_op_replacement(self, resolver, ns, expr, contents)
            }
            MutationType::RelationalOperatorReplacement => {
                rel_op_replacement(self, resolver, ns, expr, contents)
            }
            MutationType::LogicalOperatorReplacement => {
                logical_op_replacement(self, resolver, ns, expr, contents)
            }
            // Other
            MutationType::LiteralValueReplacement => {
                literal_value_replacement(self, resolver, ns, expr, contents)
            }
            MutationType::UnaryOperatorReplacement => {
                unary_op_replacement(self, resolver, ns, expr, contents)
            }
            MutationType::ExpressionValueReplacement => {
                expression_value_replacement(self, resolver, ns, expr, contents)
            }

            // Old Operators
            MutationType::ElimDelegateCall => {
                elim_delegate_mutation(self, resolver, ns, expr, contents)
            }
            _ => vec![],
        }
    }
}

impl MutationType {
    pub fn default_mutation_operators() -> Vec<MutationType> {
        vec![
            MutationType::ArithmeticOperatorReplacement,
            MutationType::BitwiseOperatorReplacement,
            MutationType::ExpressionValueReplacement,
            MutationType::ElimDelegateCall,
            MutationType::LiteralValueReplacement,
            MutationType::LogicalOperatorReplacement,
            MutationType::RelationalOperatorReplacement,
            MutationType::ShiftOperatorReplacement,
            MutationType::StatementDeletion,
            MutationType::UnaryOperatorReplacement,
        ]
    }

    pub fn short_name(&self) -> String {
        match self {
            MutationType::ArithmeticOperatorReplacement => "AOR",
            MutationType::BitwiseOperatorReplacement => "BOR",
            MutationType::ElimDelegateCall => "EDC",
            MutationType::ExpressionValueReplacement => "EVR",
            MutationType::LiteralValueReplacement => "LVR",
            MutationType::LogicalOperatorReplacement => "LOR",
            MutationType::RelationalOperatorReplacement => "ROR",
            MutationType::ShiftOperatorReplacement => "SOR",
            MutationType::StatementDeletion => "STD",
            MutationType::UnaryOperatorReplacement => "UOR",
        }
        .to_string()
    }
}

/// Find the location of an operator. This is not explicitly represented in an
/// AST node, so we have to do some digging.
fn get_op_loc(expr: &Expression, source: &Arc<str>) -> Loc {
    match expr {
        // Regular Binary operator
        Expression::Add { left, right, .. }
        | Expression::Subtract { left, right, .. }
        | Expression::Multiply { left, right, .. }
        | Expression::Divide { left, right, .. }
        | Expression::Modulo { left, right, .. }
        | Expression::BitwiseOr { left, right, .. }
        | Expression::BitwiseAnd { left, right, .. }
        | Expression::BitwiseXor { left, right, .. }
        | Expression::ShiftLeft { left, right, .. }
        | Expression::ShiftRight { left, right, .. }
        | Expression::Assign { left, right, .. }
        | Expression::More { left, right, .. }
        | Expression::Less { left, right, .. }
        | Expression::MoreEqual { left, right, .. }
        | Expression::LessEqual { left, right, .. }
        | Expression::Equal { left, right, .. }
        | Expression::NotEqual { left, right, .. }
        | Expression::Or { left, right, .. }
        | Expression::And { left, right, .. } => {
            let start = left.loc().end();
            let end = right.loc().start();
            let op = get_operator(expr);
            let substr = &source[start..end];
            let first_op_char = op.chars().next().unwrap();
            let op_offset_in_substr = substr.chars().position(|c| c == first_op_char).expect(
                format!(
                    "Error finding start/end to operator {:?} in substring {}\nExpression: {:?}\nFile: {}, Pos: {:?}",
                    op,
                    substr,
                    expr,
                    left.loc().file_no(),
                    (start, end)
                )
                .as_str(),
            );
            let op_start = start + (op_offset_in_substr as usize);
            let op_end = op_start + op.len();
            left.loc().with_start(op_start).with_end(op_end)
        }
        Expression::StringConcat { .. } | Expression::StringCompare { .. } => todo!(),

        Expression::Power { base, exp, .. } => {
            let start = base.loc().end();
            let end = exp.loc().start();
            let op = get_operator(expr);
            let substr = &source[start..end];
            let first_op_char = op.chars().next().unwrap();
            let op_offset_in_substr = substr.chars().position(|c| c == first_op_char).unwrap();
            let op_start = start + (op_offset_in_substr as usize);
            let op_end = op_start + op.len();
            base.loc().with_start(op_start).with_end(op_end)
        }
        Expression::PreIncrement { loc, expr, .. } | Expression::PreDecrement { loc, expr, .. } => {
            loc.with_end(loc.start() + get_operator(expr).len())
        }
        Expression::PostIncrement { loc, expr, .. }
        | Expression::PostDecrement { loc, expr, .. } => {
            loc.with_start(loc.end() - get_operator(expr).len())
        }

        Expression::Not { loc, expr, .. }
        | Expression::BitwiseNot { loc, expr, .. }
        | Expression::Negate { loc, expr, .. } => {
            loc.with_end(loc.start() + get_operator(expr).len())
        }

        Expression::ConditionalOperator {
            cond, true_option, ..
        } => {
            let start = cond.loc().end();
            let end = true_option.loc().start();
            let op = get_operator(expr);
            let substr = &source[start..end];
            let first_op_char = op.chars().next().unwrap();
            let op_offset_in_substr = substr.chars().position(|c| c == first_op_char).unwrap();
            let op_start = start + (op_offset_in_substr as usize);
            let op_end = op_start + op.len();
            cond.loc().with_start(op_start).with_end(op_end)
        }

        _ => panic!("No op location for {:?}", expr),
    }
}

/// Get a string representation of an operator
fn get_operator(expr: &Expression) -> &str {
    match expr {
        Expression::Add { .. } => "+",
        Expression::Subtract { .. } => "-",
        Expression::Multiply { .. } => "*",
        Expression::Divide { .. } => "/",
        Expression::Modulo { .. } => "%",
        Expression::Power { .. } => "**",
        Expression::BitwiseOr { .. } => "|",
        Expression::BitwiseAnd { .. } => "&",
        Expression::BitwiseXor { .. } => "^",
        Expression::ShiftLeft { .. } => "<<",
        Expression::ShiftRight { .. } => ">>",
        Expression::PreIncrement { .. } => "++",
        Expression::PreDecrement { .. } => "--",
        Expression::PostIncrement { .. } => "++",
        Expression::PostDecrement { .. } => "--",
        Expression::More { .. } => ">",
        Expression::Less { .. } => "<",
        Expression::MoreEqual { .. } => ">=",
        Expression::LessEqual { .. } => "<=",
        Expression::Equal { .. } => "==",
        Expression::NotEqual { .. } => "!=",
        Expression::Not { .. } => "!",
        Expression::BitwiseNot { .. } => "~",
        Expression::Negate { .. } => "-",
        Expression::ConditionalOperator { .. } => "?",
        Expression::Or { .. } => "||",
        Expression::And { .. } => "&&",
        _ => "",
    }
}

fn arith_op_replacement(
    op: &MutationType,
    file_resolver: &FileResolver,
    namespace: Rc<Namespace>,
    expr: &Expression,
    contents: &Arc<str>,
) -> Vec<Mutant> {
    let loc = expr.loc();
    let arith_op = get_operator(expr);
    let rs = vec!["+", "-", "*", "/", "**", "%"];
    let mut replacements: Vec<&str> = rs.iter().filter(|x| **x != arith_op).map(|x| *x).collect();

    if let None = loc.try_file_no() {
        return vec![];
    }
    match expr {
        Expression::BitwiseOr { .. }
        | Expression::BitwiseAnd { .. }
        | Expression::BitwiseXor { .. }
        | Expression::Divide { .. }
        | Expression::Modulo { .. }
        | Expression::Multiply { .. }
        | Expression::Subtract { .. }
        | Expression::Add { .. } => {
            let is_signed_int = if let Type::Int(_) = expr.ty() {
                true
            } else {
                false
            };
            if is_signed_int {
                // When we're signed, filter out `**`, which is illegal
                replacements = replacements
                    .iter()
                    .filter(|x| **x != "**")
                    .map(|x| *x)
                    .collect();
            }

            let op_loc = get_op_loc(expr, contents);
            replacements
                .iter()
                .map(|r| {
                    Mutant::new(
                        file_resolver,
                        namespace.clone(),
                        op_loc,
                        op.clone(),
                        arith_op.to_string(),
                        format!("{}", r),
                    )
                })
                .collect()
        }
        Expression::Power { .. } => {
            let op_loc = get_op_loc(expr, contents);
            replacements
                .iter()
                .map(|r| {
                    Mutant::new(
                        file_resolver,
                        namespace.clone(),
                        op_loc,
                        op.clone(),
                        arith_op.to_string(),
                        format!("{}", r),
                    )
                })
                .collect()
        }
        _ => vec![],
    }
}

fn bitwise_op_replacement(
    op: &MutationType,
    file_resolver: &FileResolver,
    namespace: Rc<Namespace>,
    expr: &Expression,
    source: &Arc<str>,
) -> Vec<Mutant> {
    let loc = expr.loc();
    if let None = loc.try_file_no() {
        return vec![];
    }
    match expr {
        Expression::BitwiseOr { .. } => {
            let op_loc = get_op_loc(expr, source);
            vec![Mutant::new(
                file_resolver,
                namespace,
                op_loc,
                op.clone(),
                "|".to_string(),
                "&".to_string(),
            )]
        }
        Expression::BitwiseAnd { .. } => {
            let op_loc = get_op_loc(expr, source);
            vec![Mutant::new(
                file_resolver,
                namespace,
                op_loc,
                op.clone(),
                "&".to_string(),
                "|".to_string(),
            )]
        }
        Expression::BitwiseXor { .. } => {
            let op_loc = get_op_loc(expr, source);
            vec![Mutant::new(
                file_resolver,
                namespace,
                op_loc,
                op.clone(),
                "^".to_string(),
                "&".to_string(),
            )]
        }
        _ => vec![],
    }
}

fn literal_value_replacement(
    op: &MutationType,
    file_resolver: &FileResolver,
    namespace: Rc<Namespace>,
    expr: &Expression,
    source: &Arc<str>,
) -> Vec<Mutant> {
    let loc = expr.loc();
    let orig = source[loc.start()..loc.end()].to_string();
    if let None = loc.try_file_no() {
        return vec![];
    }
    // We are only replacing BoolLiterals, NumberLiterals, and
    // RationalNumberLiterals. It's not clear what other literals we should
    // replace
    let replacements = match expr {
        Expression::BoolLiteral { value, .. } => vec![(!value).to_string()],
        Expression::NumberLiteral { ty, value, .. } => match ty {
            solang::sema::ast::Type::Address(_) => vec![],
            solang::sema::ast::Type::Int(_) => {
                if value.is_zero() {
                    vec!["-1".to_string(), "1".to_string()]
                } else {
                    vec![
                        "0".to_string(),
                        (-value).to_string(),
                        (value + BigInt::one()).to_string(),
                    ]
                }
            }
            solang::sema::ast::Type::Uint(_) => {
                if value.is_zero() {
                    vec!["1".to_string()]
                } else {
                    vec!["0".to_string(), (value + BigInt::one()).to_string()]
                }
            }
            _ => vec![],
        },
        Expression::RationalNumberLiteral { value: _, .. } => vec![],
        Expression::BytesLiteral { .. } => vec![],
        Expression::CodeLiteral { .. } => vec![],
        Expression::StructLiteral { .. } => vec![],
        Expression::ArrayLiteral { .. } => vec![],
        Expression::ConstArrayLiteral { .. } => vec![],
        _ => vec![],
    };
    let mut mutants = vec![];
    for r in replacements {
        mutants.push(Mutant::new(
            file_resolver,
            namespace.clone(),
            loc,
            op.clone(),
            orig.clone(),
            r.clone(),
        ));
    }
    mutants
}

fn logical_op_replacement(
    op: &MutationType,
    file_resolver: &FileResolver,
    namespace: Rc<Namespace>,
    expr: &Expression,
    source: &Arc<str>,
) -> Vec<Mutant> {
    let loc = expr.loc();
    if let None = loc.try_file_no() {
        return vec![];
    }

    // First, compile a list of replacements for this logical operator. Each replacement is either
    // LHS, RHS, true, or false, as well as the location of the replacing
    // expression (this is only used for RHS and LHS, since we need to compute
    // the replacement value)
    let replacements = match expr {
        Expression::And { left, right, .. } => {
            vec![("LHS", left.loc()), ("RHS", right.loc()), ("false", loc)]
        }
        Expression::Or { left, right, .. } => {
            vec![("LHS", left.loc()), ("RHS", right.loc()), ("true", loc)]
        }
        _ => {
            return vec![];
        }
    };

    // Now, apply each replacement to create a mutant
    let mut mutants = vec![];
    let orig = source[loc.start()..loc.end()].to_string();
    for (r, sub_loc) in replacements {
        mutants.push(match r {
            "LHS" | "RHS" => {
                let repl = source[sub_loc.start()..sub_loc.end()].to_string();
                Mutant::new(
                    file_resolver,
                    namespace.clone(),
                    loc,
                    op.clone(),
                    orig.clone(),
                    repl,
                )
            }
            "true" | "false" => Mutant::new(
                file_resolver,
                namespace.clone(),
                loc,
                op.clone(),
                orig.clone(),
                r.to_string(),
            ),
            _ => panic!("Illegal State"),
        });
    }
    mutants
}

fn rel_op_replacement(
    op: &MutationType,
    file_resolver: &FileResolver,
    namespace: Rc<Namespace>,
    expr: &Expression,
    source: &Arc<str>,
) -> Vec<Mutant> {
    let loc = expr.loc();
    if let None = loc.try_file_no() {
        return vec![];
    }

    let replacements = match expr {
        Expression::Less { .. } => vec!["<=", "!=", "false"],
        Expression::LessEqual { .. } => vec!["<", "==", "true"],
        Expression::More { .. } => vec![">=", "!=", "false"],
        Expression::MoreEqual { .. } => vec![">", "==", "true"],
        Expression::Equal { left, .. } => {
            // Assuming that we only need the left type to determine legal mutations
            match left.ty() {
                // The following types are orderable, so we use those for better mutation operators
                solang::sema::ast::Type::Int(_)
                | solang::sema::ast::Type::Uint(_)
                | solang::sema::ast::Type::Rational => vec!["<=", ">=", "false"],

                // The following types are not orderable, so we replace with true and false
                // TODO: Can Addresses be ordered?
                solang::sema::ast::Type::Address(_) => vec!["true", "false"],
                _ => vec!["true", "false"],
            }
        }
        Expression::NotEqual { left, .. } => {
            // Assuming that we only need the left type to determine legal mutations
            match left.ty() {
                // The following types are orderable, so we use those for better mutation operators
                solang::sema::ast::Type::Int(_)
                | solang::sema::ast::Type::Uint(_)
                | solang::sema::ast::Type::Rational => vec!["<", ">", "true"],

                // The following types are not orderable, so we replace with true and false
                // TODO: Can Addresses be ordered?
                solang::sema::ast::Type::Address(_) => vec!["true", "false"],
                _ => vec!["true", "false"],
            }
        }
        _ => return vec![],
    };

    // Now, apply the replacements. Some replacements will replace the entire
    // expression, while others will replace only the operator.
    let mut mutants = vec![];
    let expr_start = loc.start();
    let expr_end = loc.end();
    let expr_string = &source[expr_start..expr_end].to_string();

    let rel_op_loc = get_op_loc(expr, source);
    let rel_op_start = rel_op_loc.start();
    let rel_op_end = rel_op_loc.end();
    let rel_op_string = source[rel_op_start..rel_op_end].to_string();
    for r in replacements {
        mutants.push(match r {
            // true and false replacements replace the entire expression, so use
            // the expression's location (`loc`) and the expression's raw strin
            // (`expr_string`)
            "true" | "false" => Mutant::new(
                file_resolver,
                namespace.clone(),
                loc,
                op.clone(),
                expr_string.to_string(),
                r.to_string(),
            ),
            // other replacements replace only the relational operator, so use
            // the rel op location (`rel_op_loc`) and the rel op's raw string
            // (`expr_string`)
            _ => Mutant::new(
                file_resolver,
                namespace.clone(),
                rel_op_loc,
                op.clone(),
                rel_op_string.clone(),
                r.to_string(),
            ),
        });
    }
    mutants
}

fn shift_op_replacement(
    op: &MutationType,
    file_resolver: &FileResolver,
    namespace: Rc<Namespace>,
    expr: &Expression,
    source: &Arc<str>,
) -> Vec<Mutant> {
    let loc = expr.loc();
    if let None = loc.try_file_no() {
        return vec![];
    }
    match expr {
        Expression::ShiftLeft { .. } => {
            let op_loc = get_op_loc(expr, source);
            vec![Mutant::new(
                file_resolver,
                namespace,
                op_loc,
                op.clone(),
                "<<".to_string(),
                ">>".to_string(),
            )]
        }
        Expression::ShiftRight { .. } => {
            let op_loc = get_op_loc(expr, source);
            vec![Mutant::new(
                file_resolver,
                namespace,
                op_loc,
                op.clone(),
                ">>".to_string(),
                "<<".to_string(),
            )]
        }
        _ => vec![],
    }
}

fn statement_deletion(
    op: &MutationType,
    file_resolver: &FileResolver,
    namespace: Rc<Namespace>,
    stmt: &Statement,
    source: &Arc<str>,
) -> Vec<Mutant> {
    let loc = stmt.loc();
    let orig = source[loc.start()..loc.end()].to_string();
    match stmt {
        // Do not delete complex/nested statements
        Statement::Block { .. }
        | Statement::Destructure(..)
        | Statement::VariableDecl(..)
        | Statement::If(..)
        | Statement::While(..)
        | Statement::For { .. }
        | Statement::DoWhile(..)
        | Statement::Assembly(..)
        | Statement::TryCatch(..) => vec![],

        // Also, do not mutate underscore statement
        Statement::Underscore(_) => vec![],

        Statement::Expression(..)
        | Statement::Delete(..)
        | Statement::Continue(..)
        | Statement::Break(..)
        | Statement::Revert { .. }
        | Statement::Emit { .. } => vec![Mutant::new(
            file_resolver,
            namespace,
            loc,
            op.clone(),
            orig,
            "assert(true)".to_string(),
        )],

        // Returns are special: we should perform some analysis to figure out if
        // we can delete this without making an invalid program. For now we
        // delete and hope for the best :)
        Statement::Return(..) => vec![Mutant::new(
            file_resolver,
            namespace,
            loc,
            op.clone(),
            orig,
            "assert(true)".to_string(),
        )],
    }
}

fn unary_op_replacement(
    op: &MutationType,
    file_resolver: &FileResolver,
    namespace: Rc<Namespace>,
    expr: &Expression,
    source: &Arc<str>,
) -> Vec<Mutant> {
    let loc = expr.loc();
    let unary_op = get_operator(expr);
    let rs = vec!["-", "~"];
    let replacements: Vec<&&str> = rs.iter().filter(|x| **x != unary_op).collect();

    if let None = loc.try_file_no() {
        return vec![];
    }
    let muts = match expr {
        Expression::BitwiseNot { .. } | Expression::Negate { .. } => {
            let op_loc = get_op_loc(expr, source);
            let muts = replacements
                .iter()
                .map(|r| {
                    Mutant::new(
                        file_resolver,
                        namespace.clone(),
                        op_loc,
                        op.clone(),
                        "~".to_string(),
                        format!(" {} ", r),
                    )
                })
                .collect();
            muts
        }
        _ => vec![],
    };
    muts
}

#[allow(dead_code)]
fn elim_delegate_mutation(
    op: &MutationType,
    file_resolver: &FileResolver,
    namespace: Rc<Namespace>,
    expr: &Expression,
    source: &Arc<str>,
) -> Vec<Mutant> {
    // TODO: implement
    match expr {
        Expression::ExternalFunctionCallRaw {
            loc,
            ty: CallTy::Delegate,
            address,
            ..
        } => {
            // Ugh, okay, so we need to do messy string manipulation to get the
            // location of the function name because that isn't tracked in the
            // AST. The idea is that we start scanning from the right of the
            // address (e.g., `foo` in `foo.bar()`), and look for the first
            // index of `delegatecall`. We then add an offset of 12 (length of
            // "delegatecall").
            let addr_loc = address.loc();
            let idx = addr_loc.end() + 1;
            let no_address = &source[idx..loc.end()];
            let delegate_call_start = idx + no_address.find("delegatecall").unwrap();
            let delegate_call_end = delegate_call_start + 12;

            vec![Mutant::new(
                &file_resolver,
                namespace,
                Loc::File(loc.file_no(), delegate_call_start, delegate_call_end),
                op.clone(),
                "delegatecall".to_string(),
                "call".to_string(),
            )]
        }
        _ => vec![],
    }
}

#[allow(dead_code)]
fn expression_value_replacement(
    _op: &MutationType,
    _file_resolver: &FileResolver,
    _namespace: Rc<Namespace>,
    _expr: &Expression,
    _source: &Arc<str>,
) -> Vec<Mutant> {
    // TODO: implement
    vec![]
}

/// This testing module defines and uses the testing infrastructure, allowing
/// for varying degrees of testing flexibility.
///
/// First, we define two types of assert functions:
///
/// 1. `assert_num_mutants_for_XXX`: make an assertion for the number of mutants
///    generated by a provided set of operators for the given code
///
/// 2. `assert_exact_mutants_for_XXX`: make an assertion on the exact mutants
///    generated by a provided set of operators for the given code
///
/// Next, we define two forms of code to make assertions about:
///
/// 1. **Statements:** we represent a list of statements as a `Vec<&str>`, and
///    this makes it easy to write a simple program to mutate, e.g.,
///
///    ```
///    vec!["uint256 a = 1;", "uint256 b = 2;", "uint256 c = a + b;"]
///    ```
///
///    Functions `assert_exact_mutants_for_statements()` and
///    `assert_num_mutants_for_statements()` use this program form to make
///    assertions about mutations
///
/// 2. **Full Source:** sometimes we want to be able to specify a full source file.
///    This is more verbose than using statements (as we need to write more
///    boilerplate), but we have maximal flexibility in the code we are mutating.
///
///    Funcctions `assert_exact_mutants_for_source()` and
///    `assert_num_mutants_for_source()` use this program form to make
///    assertions about mutations
#[cfg(test)]
mod test {
    use crate::test_util::*;
    use crate::{MutationType, Mutator, MutatorConf, Solc};
    use solang::file_resolver::FileResolver;
    use std::collections::HashSet;
    use std::error;
    use std::path::PathBuf;
    use tempfile::Builder;

    #[test]
    pub fn test_elim_delegate_mutation() -> Result<(), Box<dyn error::Error>> {
        let ops = vec![
            MutationType::ElimDelegateCall,
            MutationType::ArithmeticOperatorReplacement,
        ];
        // TODO: how should I test this?
        let code = "\
// SPDX-License-Identifier: GPL-3.0-only
pragma solidity ^0.8.9;

contract B {
    uint public num;
    address public sender;
    uint public value;

    function setVars(uint _num) public payable {
        usize c = 1 + 2;

        num = _num;
        sender = msg.sender;
        value = msg.value;
    }
}

contract A {
    uint public num;
    address public sender;
    uint public value;
    bool public delegateSuccessful;
    bytes public myData;
    

    function setVars(address _contract, uint _num) public payable {
        (bool success, bytes memory data) = _contract.delegatecall(
            abi.encodeWithSignature(\"setVars(uint256)\", _num)
        );
	delegateSuccessful = success;
	myData = data;
    }
}
";
        let expected = vec!["call"];
        assert_exact_mutants_for_source(code, &ops, &expected);
        Ok(())
    }

    #[test]
    fn test_aor() {
        let ops = vec![MutationType::ArithmeticOperatorReplacement];
        assert_exact_mutants_for_statements(
            &vec!["uint256 a", "uint256 b"],
            &vec!["uint256 c = a + b;"],
            &ops,
            &vec!["-", "*", "/", "**", "%"],
        );
        assert_exact_mutants_for_statements(
            &vec!["int256 a", "int256 b"],
            &vec!["int256 c = a + b;"],
            &ops,
            &vec!["-", "*", "/", "%"],
        );

        assert_exact_mutants_for_statements(
            &vec!["uint256 a", "uint256 b"],
            &vec!["uint256 c = a - b;"],
            &ops,
            &vec!["+", "*", "/", "**", "%"],
        );

        assert_exact_mutants_for_statements(
            &vec!["uint256 a", "uint256 b"],
            &vec!["uint256 c = a * b;"],
            &ops,
            &vec!["+", "-", "/", "**", "%"],
        );
        assert_exact_mutants_for_statements(
            &vec!["uint256 a", "uint256 b"],
            &vec!["uint256 c = a ** b;"],
            &ops,
            &vec!["+", "-", "/", "*", "%"],
        );
        assert_exact_mutants_for_statements(
            &vec!["uint256 a", "uint256 b"],
            &vec!["uint256 c = a % b;"],
            &ops,
            &vec!["+", "-", "/", "*", "**"],
        );
    }

    #[test]
    fn test_bor() {
        let ops = vec![MutationType::BitwiseOperatorReplacement];
        assert_exact_mutants_for_statements(
            &vec!["uint256 a", "uint256 b"],
            &vec!["uint256 c = a | b;"],
            &ops,
            &vec!["&"],
        );

        assert_exact_mutants_for_statements(
            &vec!["uint256 a", "uint256 b"],
            &vec!["uint256 c = a & b;"],
            &ops,
            &vec!["|"],
        );

        assert_exact_mutants_for_statements(
            &vec!["uint256 a", "uint256 b"],
            &vec!["uint256 c = a ^ b;"],
            &ops,
            &vec!["&"],
        );
    }

    #[test]
    fn test_lvr() {
        let ops = vec![MutationType::LiteralValueReplacement];
        // Numbers
        assert_exact_mutants_for_statements(
            &vec!["uint256 a", "uint256 b"],
            &vec!["uint256 c = a * b + 11;"],
            &ops,
            &vec!["0", "12"],
        );
        assert_exact_mutants_for_statements(
            &vec!["uint256 a", "uint256 b"],
            &vec!["uint256 c = a * b + 0;"],
            &ops,
            &vec!["1"],
        );
        assert_exact_mutants_for_statements(
            &vec!["int256 a", "int256 b"],
            &vec!["int256 c = a * b + 11;"],
            &ops,
            &vec!["0", "-11", "12"],
        );
        assert_exact_mutants_for_statements(
            &vec!["int256 a", "int256 b"],
            &vec!["int256 c = a * b + 0;"],
            &ops,
            &vec!["1", "-1"],
        );

        // Booleans
        assert_exact_mutants_for_statements(&vec![], &vec!["bool b = true;"], &ops, &vec!["false"]);
        assert_exact_mutants_for_statements(&vec![], &vec!["bool b = false;"], &ops, &vec!["true"]);
    }

    #[test]
    fn test_lor() {
        let ops = vec![MutationType::LogicalOperatorReplacement];

        assert_exact_mutants_for_statements(
            &vec!["bool a", "bool b"],
            &vec!["bool c = a || b;"],
            &ops,
            &vec!["a", "b", "true"],
        );

        assert_exact_mutants_for_statements(
            &vec!["bool a", "bool b"],
            &vec!["bool c = a && b;"],
            &ops,
            &vec!["a", "b", "false"],
        );
    }

    #[test]
    fn test_ror() {
        assert_exact_mutants_for_statements(
            &vec!["uint256 a", "uint256 b"],
            &vec!["if (a < b) {}"],
            &vec![MutationType::RelationalOperatorReplacement],
            &vec!["<=", "false", "!="],
        );

        assert_exact_mutants_for_statements(
            &vec!["uint256 a", "uint256 b"],
            &vec!["if (a == b) {}"],
            &vec![MutationType::RelationalOperatorReplacement],
            &vec!["<=", "false", ">="],
        );

        assert_exact_mutants_for_statements(
            &vec!["uint256 a", "uint256 b"],
            &vec!["if (a > b) {}"],
            &vec![MutationType::RelationalOperatorReplacement],
            &vec!["!=", "false", ">="],
        );

        assert_exact_mutants_for_statements(
            &vec!["uint256 a", "uint256 b"],
            &vec!["if (a <= b) {}"],
            &vec![MutationType::RelationalOperatorReplacement],
            &vec!["<", "true", "=="],
        );

        assert_exact_mutants_for_statements(
            &vec!["uint256 a", "uint256 b"],
            &vec!["if (a != b) {}"],
            &vec![MutationType::RelationalOperatorReplacement],
            &vec!["<", "true", ">"],
        );

        assert_exact_mutants_for_statements(
            &vec!["uint256 a", "uint256 b"],
            &vec!["if (a >= b) {}"],
            &vec![MutationType::RelationalOperatorReplacement],
            &vec!["==", "true", ">"],
        );

        assert_exact_mutants_for_statements(
            &vec!["bool a", "bool b"],
            &vec!["if (a != b) {}"],
            &vec![MutationType::RelationalOperatorReplacement],
            &vec!["true", "false"],
        );

        assert_exact_mutants_for_statements(
            &vec!["bool a", "bool b"],
            &vec!["if (a == b) {}"],
            &vec![MutationType::RelationalOperatorReplacement],
            &vec!["true", "false"],
        );
    }

    #[test]
    fn test_std() {
        let ops = vec![MutationType::StatementDeletion];
        assert_exact_mutants_for_statements(
            &vec!["uint256 a", "uint256 b"],
            &vec!["uint256 c = a + b;"],
            &ops,
            &vec![],
        );
        assert_exact_mutants_for_statements(
            &vec!["uint256 a", "uint256 b"],
            &vec!["uint256 c;", "c = a + b;"],
            &ops,
            &vec!["assert(true)"],
        );
        assert_exact_mutants_for_statements(
            &vec!["bool a"],
            &vec!["while (a) { continue; }"],
            &ops,
            &vec!["assert(true)"],
        );
        assert_exact_mutants_for_statements(
            &vec!["bool a"],
            &vec!["while (a) { break; }"],
            &ops,
            &vec!["assert(true)"],
        );
        assert_exact_mutants_for_statements(
            &vec!["bool a"],
            &vec!["revert();"],
            &ops,
            &vec!["assert(true)"],
        );
        // TODO: add a test for `delete expr`
        // TODO: add a test for `emit ...`
    }

    #[test]
    fn test_sor() {
        let ops = vec![MutationType::ShiftOperatorReplacement];
        assert_exact_mutants_for_statements(
            &vec!["uint256 a", "uint256 b"],
            &vec!["uint256 c = a << b;"],
            &ops,
            &vec![">>"],
        );

        assert_exact_mutants_for_statements(
            &vec!["uint256 a", "uint256 b"],
            &vec!["uint256 c = a >> b;"],
            &ops,
            &vec!["<<"],
        );
    }

    #[test]
    fn test_uor() {
        let ops = vec![MutationType::UnaryOperatorReplacement];
        assert_exact_mutants_for_statements(
            &vec!["int256 a"],
            &vec!["int256 b = -a;"],
            &ops,
            &vec!["~"],
        );
        assert_exact_mutants_for_statements(
            &vec!["int256 a"],
            &vec!["int256 b = ~a;"],
            &ops,
            &vec!["-"],
        );
    }

    #[allow(dead_code)]
    fn assert_num_mutants_for_statements(
        params: &Vec<&str>,
        statements: &Vec<&str>,
        ops: &Vec<MutationType>,
        expected: usize,
    ) {
        let mutator = apply_mutation_to_statements(statements, params, None, ops).unwrap();
        assert_eq!(
            expected,
            mutator.mutants().len(),
            "Error: applied ops\n   -> {:?}\nto program\n  -> {:?}\nat {:?} for more info",
            ops,
            statements.join("   "),
            mutator.filenames().join(" ")
        );
    }

    #[allow(dead_code)]
    fn assert_exact_mutants_for_statements(
        params: &Vec<&str>,
        statements: &Vec<&str>,
        ops: &Vec<MutationType>,
        expected: &Vec<&str>,
    ) {
        let mutator = apply_mutation_to_statements(statements, params, None, ops).unwrap();
        let expected_set: HashSet<&str> = expected.iter().map(|s| s.trim()).collect();
        let actuals_set: HashSet<&str> = mutator
            .mutants()
            .iter()
            .map(|m| m.repl.as_str().trim())
            .collect();
        let expected_str = expected_set
            .iter()
            .cloned()
            .collect::<Vec<&str>>()
            .join(", ");
        let actuals_str = actuals_set
            .iter()
            .cloned()
            .collect::<Vec<&str>>()
            .join(", ");
        let program =
            ansi_term::Color::Yellow.paint(format!("```\n{}\n```", statements.join(";\n")));
        assert_eq!(
            expected.len(),
            mutator.mutants().len(),
            "Error: applied ops:
               -> {:?}
            to program:\n{}\n
            [+] Expected mutants: {}
            [X] Actual mutants: {}
            See {} for more info",
            ops,
            program,
            ansi_term::Color::Green.paint(expected_str),
            ansi_term::Color::Red.paint(actuals_str),
            mutator.filenames().join(" ")
        );

        assert_eq!(
            actuals_set,
            expected_set,
            "Error: applied ops:
               -> {:?}
            to program:\n{}\n
            [+] Expected mutants: {}
            [X]   Actual mutants: {}
            See {} for more info",
            ops,
            program,
            ansi_term::Color::Green.paint(expected_str),
            ansi_term::Color::Red.paint(actuals_str),
            mutator.filenames().join(" ")
        );
    }

    fn apply_mutation_to_statements(
        statements: &Vec<&str>,
        params: &Vec<&str>,
        returns: Option<&str>,
        ops: &Vec<MutationType>,
    ) -> Result<Mutator, Box<dyn error::Error>> {
        let source = wrap_and_write_solidity_to_temp_file(statements, params, returns).unwrap();
        let prefix = format!(
            "gambit-compile-dir-{}",
            source.file_name().unwrap().to_str().unwrap()
        );
        let outdir = Builder::new()
            .prefix(prefix.as_str())
            .rand_bytes(5)
            .tempdir_in(source.parent().unwrap())?;
        let mut mutator = make_mutator(ops, source, outdir.into_path());
        mutator
            .file_resolver
            .add_import_path(&PathBuf::from("/"))
            .unwrap();
        let sources = mutator.filenames().clone();
        mutator.mutate(sources)?;

        Ok(mutator)
    }

    fn _assert_num_mutants_for_source(source: &str, ops: &Vec<MutationType>, expected: usize) {
        let mutator = apply_mutation_to_source(source, ops).unwrap();
        assert_eq!(
            expected,
            mutator.mutants().len(),
            "Error: applied ops\n   -> {:?}\nto program\n  -> {:?}\n\nSee {:?} for more info",
            ops,
            source,
            mutator.filenames().join(" ")
        );
    }

    fn assert_exact_mutants_for_source(
        source: &str,
        ops: &Vec<MutationType>,
        expected: &Vec<&str>,
    ) {
        let mutator = apply_mutation_to_source(source, ops).unwrap();
        assert_eq!(
            expected.len(),
            mutator.mutants().len(),
            "Error: applied ops\n   -> {:?}\nto program\n  -> {:?}\nat {} for more info",
            ops,
            source,
            mutator.filenames().join(" ")
        );

        let actuals: HashSet<&str> = mutator.mutants().iter().map(|m| m.repl.as_str()).collect();
        let expected: HashSet<&str> = expected.iter().map(|s| *s).collect();
        assert_eq!(actuals, expected);
    }

    fn apply_mutation_to_source(
        source: &str,
        ops: &Vec<MutationType>,
    ) -> Result<Mutator, Box<dyn error::Error>> {
        let source = write_solidity_to_temp_file(source.to_string()).unwrap();
        let outdir = Builder::new()
            .prefix("gambit-compile-dir")
            .rand_bytes(5)
            .tempdir()?;
        let mut mutator = make_mutator(ops, source.clone(), outdir.into_path());
        // let source_os_str = source.as_os_str();
        // println!("source: {:?}", source_os_str);
        // let ns = parse_and_resolve(
        //     source_os_str,
        //     &mut mutator.file_resolver,
        //     solang::Target::EVM,
        // );
        // println!("FUNCTIONS");
        // println!("ns: {:?}", ns.files);
        // for function in ns.functions {
        //     println!("[{}]:\n", function.name);
        //     for (i, s) in function.body.iter().enumerate() {
        //         println!("  {}: {:?}", i + 1, &s);
        //     }
        // }
        mutator
            .file_resolver
            .add_import_path(&PathBuf::from("/"))
            .unwrap();
        let sources = mutator.filenames().clone();
        mutator.mutate(sources)?;

        Ok(mutator)
    }

    /// Create a mutator for a single file, creating required components (e.g.,
    /// Solc, creating Sources and rapping them in a Vec<Rc<Source>>, etc)
    fn make_mutator(ops: &Vec<MutationType>, filename: PathBuf, outdir: PathBuf) -> Mutator {
        let conf = MutatorConf {
            mutation_operators: ops.clone(),
            funcs_to_mutate: None,
            contract: None,
        };

        let sources = vec![filename.to_str().unwrap().to_string()];
        let solc = Solc::new("solc".into(), PathBuf::from(outdir));
        let mut cache = FileResolver::new();
        cache.add_import_path(&PathBuf::from("/")).unwrap();
        Mutator::new(conf, cache, sources, solc)
    }
}
