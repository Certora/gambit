use std::{
    error::Error,
    path::{Path, PathBuf},
};

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

type CommandOutput = (Option<i32>, Vec<u8>, Vec<u8>);

/// Utility for invoking any command `cmd` with `args`.
/// Returns the tuple (`status.code`, `stdout` and `stderr`).
pub fn invoke_command(cmd: &str, args: Vec<&str>) -> Result<CommandOutput, Box<dyn Error>> {
    let out = std::process::Command::new(cmd)
        .args(args.iter().map(|a| a.to_string()))
        .output();
    if out.is_err() {
        panic!("Failed to invoke {}.", cmd);
    } else {
        let res = out.ok().unwrap();
        Ok((res.status.code(), res.stdout, res.stderr))
    }
}

/// Given a path, returns the Normal components of the path as a PathBuf.
/// This includes the leaf of the path.
pub fn get_path_normals(path: &str) -> Option<PathBuf> {
    let path = Path::new(path);
    let comps = path.components().collect::<Vec<_>>();
    let normals: Vec<PathBuf> = comps
        .iter()
        .filter(|c| matches!(c, std::path::Component::Normal(_)))
        .map(|c| Path::new(c).to_path_buf())
        .collect();
    if normals.is_empty() {
        return None;
    }
    let mut root = normals[0].clone();
    for item in normals.iter().skip(1) {
        root.push(item);
    }
    Some(root.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_convert_path1() {
        let path1 = "../../../gambit/out/foo/TenPower.sol0.sol";
        let res1 = get_path_normals(path1).unwrap();
        assert_eq!(
            res1,
            Path::new("gambit/out/foo/TenPower.sol0.sol").to_path_buf()
        );
    }

    #[test]
    fn test_convert_path2() {
        let path2 = "gambit/out/foo/TenPower.sol0.sol";
        let res2 = get_path_normals(path2).unwrap();
        assert_eq!(
            res2,
            Path::new("gambit/out/foo/TenPower.sol0.sol").to_path_buf()
        );
    }

    #[test]
    fn test_convert_path3() {
        let path3 = "/gambit/out/foo/TenPower";
        let res3 = get_path_normals(path3).unwrap();
        assert_eq!(res3, Path::new("gambit/out/foo/TenPower").to_path_buf());
    }

    #[test]
    fn test_convert_path4() {
        let path3 = "";
        let res3 = get_path_normals(path3);
        assert_eq!(res3, None);
    }

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
}
