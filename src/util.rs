use std::{
    error::Error,
    fs::File,
    io::Read,
    path::{Component, Path, PathBuf},
};

use ansi_term::{ANSIGenericString, Color, Style};
use solang::sema::ast::Statement;

static EQUAL: &str = "=";
pub static DEFAULT_GAMBIT_OUTPUT_DIRECTORY: &str = "gambit_out";

pub fn default_gambit_output_directory() -> String {
    DEFAULT_GAMBIT_OUTPUT_DIRECTORY.to_string()
}

/// Given two strings, resolve the second one (`target`) w.r.t. the first (`against`).
pub fn resolve_path_from_str(against: &str, target: &str) -> String {
    let ag = PathBuf::from(&against);
    let tgt = PathBuf::from(&target);
    return resolve_against_parent(&ag, &tgt)
        .to_str()
        .unwrap()
        .to_string();
}

/// Resolve the path `target` against `against`'s parent.
pub fn resolve_against_parent(against: &Path, target: &Path) -> PathBuf {
    let par_of_agnst = against.parent();
    if let Some(par) = par_of_agnst {
        let mut against_par = par.to_path_buf();
        against_par.push(target);
        against_par
    } else {
        target.to_path_buf()
    }
}

/// Given a `line`, get the indentation in terms of
/// a string of white spaces.
pub fn get_indent(line: &str) -> String {
    let mut res = String::new();
    for c in line.chars() {
        if c.is_whitespace() {
            res += &c.to_string();
        } else {
            break;
        }
    }
    res
}

/// Resolve a remapping path.
///
/// A remapping string is of the form `@aave=path/to/aave-gho/node_modules/@aave`.
/// This is of the form `@NAME=PATH`. This function separates `PATH` from the
/// remapping string, resolves `PATH` with respect to the optional
/// `resolve_against` (defaults to '.' if `None` is provided), canonicalizes the
/// resolved path, and repackages a new canonical remapping string.
///
/// # Arguments
///
/// * `remap_str` - the remapping string whose path needs to be resolved
/// * `resolve_against` - an optional path that `remap_str`'s PATH will be
///   resolved against---by default (i.e., if `None` is provided), this is
///   treated as `"."`
pub fn repair_remapping(remap_str: &str, resolve_against: Option<&str>) -> String {
    log::debug!(
        "Repairing remap {} against path {:?}",
        remap_str,
        &resolve_against
    );
    let against_path_str = if let Some(p) = resolve_against {
        p
    } else {
        "."
    };
    let parts: Vec<&str> = remap_str.split(EQUAL).collect();
    assert_eq!(
        parts.len(),
        2,
        "repair_remappings: remapping must have the shape @foo=bar/baz/blip, please check {}.",
        remap_str
    );
    let lhs = parts[0];
    let rhs = parts[1];
    let resolved_path = PathBuf::from(against_path_str)
        .join(rhs)
        .canonicalize()
        .unwrap();
    let resolved = resolved_path.to_str().unwrap();
    let result = lhs.to_owned() + EQUAL + resolved;
    log::debug!("Repaired to {}", result);
    result
}

type CommandOutput = (Option<i32>, Vec<u8>, Vec<u8>);

/// Utility for invoking any command `cmd` with `args`.
/// Returns the tuple (`status.code`, `stdout` and `stderr`).
pub fn invoke_command(cmd: &str, args: Vec<&str>) -> Result<CommandOutput, Box<dyn Error>> {
    let out = std::process::Command::new(cmd)
        .args(args.iter().map(|a| a.to_string()))
        .output();
    if out.is_err() {
        panic!("Failed to invoke {cmd}.");
    } else {
        let res = out.ok().unwrap();
        Ok((res.status.code(), res.stdout, res.stderr))
    }
}

pub fn read_source(orig_path: &Path) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut source = Vec::new();
    let mut f = File::open(orig_path)?;
    f.read_to_end(&mut source)?;
    Ok(source)
}

/// Given a unified diff string, print a colorized version of this string to
/// stdout.
///
/// This function colorizes the diff line by line based on the type of the line.
/// Lines in a unified diff can start with:
///
/// * `+`: this indicates an _addition_ of code; we color these lines _green_
/// * `-`: this indicates a _deletion_ of code; we color these lines _red_
/// * `@`: this indicates file location data (line number, etc); we color these
///   lines _cyan_
/// * Anything else: we do not format other lines
pub fn print_colorized_unified_diff(diff: String) {
    let clines: Vec<ANSIGenericString<str>> = diff
        .lines()
        .map(|line| {
            if line.starts_with('-') {
                Color::Red.paint(line)
            } else if line.starts_with('+') {
                Color::Green.paint(line)
            } else if line.starts_with('@') {
                Color::Cyan.paint(line)
            } else {
                Style::default().paint(line)
            }
        })
        .collect();
    for line in clines {
        println!("{}", line);
    }
}

/// Simplify a path for readability. This first canonicalizes a path, then tries
/// to transform it into a relative path from the current working directory:
/// this is successful only when the path is a descendend of the CWD (i.e., when
/// `PathBuf::from(".").canonicalize()` is a prefix of the canonicalized
/// `path`). If this is successful, this function returns the relative path.
/// Otherwise, return the canonical path.
pub fn simplify_path(path: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let can_path = path.canonicalize()?;
    let rel_path = match can_path.strip_prefix(PathBuf::from(".").canonicalize()?) {
        Ok(p) => p.to_path_buf(),
        Err(_) => can_path,
    };
    Ok(rel_path)
}

/// Make a relative path from the base path. This function takes two paths,
/// canonicalizes them both, and then strips the `base` path as a prefix from
/// `path`. If either path doesn't exist, or if the canonicalized `base` is not
/// a prefix of the canonicalized `path`, return an error.
pub fn rel_path_from_base(path: &Path, base: &Path) -> Result<PathBuf, Box<dyn Error>> {
    let can_base = base.canonicalize()?;
    let can_path = path.canonicalize()?;
    match can_path.strip_prefix(can_base) {
        Ok(p) => Ok(p.to_path_buf()),
        Err(e) => Err(Box::new(e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_indent1() {
        let s = "";
        let res = get_indent(s);
        assert_eq!(res, "")
    }

    #[test]
    fn test_get_indent2() {
        let s = "   ";
        let res = get_indent(s);
        assert_eq!(res, "   ")
    }

    #[test]
    fn test_get_indent3() {
        let s = "   Hello World";
        let res = get_indent(s);
        assert_eq!(res, "   ")
    }

    #[test]
    fn test_get_indent4() {
        let s = "   Hello    World   ";
        let res = get_indent(s);
        assert_eq!(res, "   ")
    }

    #[test]
    fn test_resolve_from_string1() {
        let s = "foo";
        let t = "bar";
        assert_eq!(
            resolve_path_from_str(s, t),
            PathBuf::from("bar").to_str().unwrap()
        )
    }

    #[test]
    fn test_resolve_from_string2() {
        let s = ".";
        let t = "../baz";
        assert_eq!(
            resolve_path_from_str(s, t),
            PathBuf::from("../baz").to_str().unwrap()
        )
    }

    #[test]
    fn test_resolve_from_string3() {
        let s = "../../fun";
        let t = ".";
        assert_eq!(
            resolve_path_from_str(s, t),
            PathBuf::from("../../.").to_str().unwrap()
        )
    }

    #[test]
    fn test_resolve_from_string4() {
        let s = "/User/foo/bar";
        let t = "bar";
        assert_eq!(
            resolve_path_from_str(s, t),
            PathBuf::from("/User/foo/bar").to_str().unwrap()
        )
    }

    #[test]
    fn test_resolve_from_string5() {
        let s = "../../../foo";
        let t = "../../bar/bar/blip.sh";
        assert_eq!(
            resolve_path_from_str(s, t),
            PathBuf::from("../../../../../bar/bar/blip.sh")
                .to_str()
                .unwrap()
        )
    }

    #[test]
    fn test_resolve_from_string6() {
        let s = "../../../foo/bing.json";
        let t = "../../bar/bar/blip.sh";
        assert_eq!(
            resolve_path_from_str(s, t),
            PathBuf::from("../../../foo/../../bar/bar/blip.sh")
                .to_str()
                .unwrap()
        )
    }

    // Note: I'm ignoring the following two tests. I've updated
    // `repair_remapping` to use `fs::canonicalize()` which requires a path to
    // exist. This makes writing these tests a bit trickier.
    #[ignore]
    #[test]
    fn test_remapping1() {
        let aave = "@aave=../../../Test/aave-gho/node_modules/@aave";
        let base = ".";
        let res = "@aave=../../../Test/aave-gho/node_modules/@aave";
        assert_eq!(repair_remapping(aave, Some(base)), res)
    }

    #[ignore]
    #[test]
    fn test_remapping2() {
        let aave = "@aave=/Test/aave-gho/node_modules/@aave";
        let base = "/foo/bar";
        let res = "@aave=/Test/aave-gho/node_modules/@aave";
        assert_eq!(repair_remapping(aave, Some(base)), res)
    }

    use crate::simplify_path;
    use std::path::PathBuf;

    #[test]
    pub fn test_simplify_path() {
        assert_simplifed_path("benchmarks/10Power", "benchmarks/10Power");
        assert_simplifed_path("benchmarks/10Power", "benchmarks/../benchmarks/10Power");
        assert_simplifed_path("benchmarks", "benchmarks/../benchmarks/10Power/..");
        assert_simplifed_path("", "benchmarks/../benchmarks/10Power/../..");
    }

    /// Helper function to assert simplified paths
    fn assert_simplifed_path(expected: &str, path: &str) {
        let path = PathBuf::from(path);
        let expected = PathBuf::from(expected);
        let simplified = simplify_path(&path);
        assert!(
            simplified.is_ok(),
            "Expected Ok(...) but found {:?}",
            simplified
        );
        let simplified = simplified.unwrap();
        assert_eq!(expected, simplified);
    }
}

/// Normalize a path without checking if it exists. Taken from Cargo:
/// https://github.com/rust-lang/cargo/blob/fede83ccf973457de319ba6fa0e36ead454d2e20/src/cargo/util/paths.rs#L61
pub fn normalize_path(path: &Path) -> PathBuf {
    let mut components = path.components().peekable();
    let mut ret = if let Some(c @ Component::Prefix(..)) = components.peek().cloned() {
        components.next();
        PathBuf::from(c.as_os_str())
    } else {
        PathBuf::new()
    };

    for component in components {
        match component {
            Component::Prefix(..) => unreachable!(),
            Component::RootDir => {
                ret.push(component.as_os_str());
            }
            Component::CurDir => {}
            Component::ParentDir => {
                ret.pop();
            }
            Component::Normal(c) => {
                ret.push(c);
            }
        }
    }
    ret
}

pub fn normalize_mutation_operator_name(op_name: &String) -> String {
    let tmp = op_name.to_lowercase();
    let tmp = tmp.replace("_", "-");
    let op_name_lower = tmp.as_str();
    match op_name_lower {
        "aor" | "arithmetic-operator-replacement" => "logical-operator-replacement",
        "bor" | "bitwise-operator-replacement" => "bitwise-operator-replacement",
        "evr" | "expression-value-replacement" => "expression-value-replacement",
        "lor" | "logical-operator-replacement" => "logical-operator-replacement",
        "lvr" | "literal-value-replacement" => "literal-value-replacement",
        "ror" | "relational-operator-replacement" => "relational-operator-replacement",
        "sor" | "shift-operator-replacement" => "shift-operator-replacement",
        "std" | "statement-deletion" => "statement-deletion",
        "uor" | "unary-operator-replacement" => "unary-operator-replacement",
        _ => op_name_lower,
    }
    .to_string()
}

/// Return a simple string representation of the type of statement
pub fn statement_type(stmt: &Statement) -> &str {
    match stmt {
        Statement::Block { .. } => "Block",
        Statement::VariableDecl(..) => "VariableDecl",
        Statement::If(..) => "If",
        Statement::While(..) => "While",
        Statement::For { .. } => "For",
        Statement::DoWhile(..) => "DoWhile",
        Statement::Expression(..) => "Expression",
        Statement::Delete(..) => "Delete",
        Statement::Destructure(..) => "Destructure",
        Statement::Continue(..) => "Continue",
        Statement::Break(_) => "Break",
        Statement::Return(_, _) => "Return",
        Statement::Revert { .. } => "Revert",
        Statement::Emit { .. } => "Emit",
        Statement::TryCatch(_, _, _) => "TryCatch",
        Statement::Underscore(_) => "Underscore",
        Statement::Assembly(_, _) => "Assembly",
    }
}
