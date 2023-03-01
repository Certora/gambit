use std::{error::Error, io, path::PathBuf};

static EQUAL: &str = "=";

/// Given two strings, resolve the second one (`target`) w.r.t. the first (`against`).
pub fn resolve_path_from_str(against: &str, target: &str) -> String {
    let ag = PathBuf::from(&against);
    let tgt = PathBuf::from(&target);
    return resolve_against_parent(ag, tgt)
        .to_str()
        .unwrap()
        .to_string();
}

/// Resolve the path `target` against `against`'s parent.
pub fn resolve_against_parent(against: PathBuf, target: PathBuf) -> PathBuf {
    let par_of_agnst = against.parent();
    if let Some(par) = par_of_agnst {
        let mut against_par = par.to_path_buf();
        against_par.push(target);
        against_par
    } else {
        target
    }
}

/// Simply calls `fs::canonicalize()` but first converts `path` to a PathBuf.
pub fn canon_path_from_str(path: &str) -> io::Result<PathBuf> {
    let path = PathBuf::from(path);
    let canon = path.canonicalize()?;
    Ok(canon)
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

/// We need to fix solc remappings because they are simple strings that look like:
/// `@aave=path/to/aave-gho/node_modules/@aave`.
/// This involves some string manipulation which isn't great.
pub fn repair_remapping(remap: &str, against_path_str: &str) -> String {
    let parts: Vec<&str> = remap.split(EQUAL).collect();
    assert_eq!(
        parts.len(),
        2,
        "repair_remappings: remapping must have the shape @foo=bar/baz/blip, please check {}.",
        remap
    );
    let lhs = parts[0];
    let rhs = parts[1];
    let fixed_rhs = resolve_path_from_str(against_path_str, rhs);
    lhs.to_owned() + EQUAL + &fixed_rhs
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

    #[test]
    fn test_remapping1() {
        let aave = "@aave=../../../Test/aave-gho/node_modules/@aave";
        let base = ".";
        let res = "@aave=../../../Test/aave-gho/node_modules/@aave";
        assert_eq!(repair_remapping(aave, base), res)
    }

    #[test]
    fn test_remapping2() {
        let aave = "@aave=/Test/aave-gho/node_modules/@aave";
        let base = "/foo/bar";
        let res = "@aave=/Test/aave-gho/node_modules/@aave";
        assert_eq!(repair_remapping(aave, base), res)
    }
}
