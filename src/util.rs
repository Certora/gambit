use core::hash::Hash;
use std::{
    collections::HashMap,
    error::Error,
    path::{Path, PathBuf}, vec,
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

/// Given a vec of pairs of type `(T1, T2)` and a vec of type `T1`, generate a hashmap from T1 keys to `Vec<T2>`.
pub fn vec_pair_to_map<T1, T2>(vecs_of_pairs: &Vec<(T1, T2)>, es: &Vec<T1>) -> HashMap<T1, Vec<T2>>
where
    T1: Hash + Clone + std::cmp::Eq + PartialEq,
    T2: Clone,
{
    let mut map: HashMap<T1, Vec<T2>> = HashMap::new();
    for e in es {
        let mut vals: Vec<T2> = vec![];
        for (k, v) in vecs_of_pairs {
            if e == k {
                vals.push(v.clone());
            }
        }
        map.insert(e.clone(), vals);
    }
    map
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
    fn test_vec_pair_to_map1() {
        let vs: Vec<(i32, i32)> = vec![];
        let es = vec![1, 2, 3];
        let mp = vec_pair_to_map(&vs, &es);
        assert_eq!(mp, HashMap::from([(3, vec![]), (2, vec![]), (1, vec![])]))
    }

    #[test]
    fn test_vec_pair_to_map2() {
        let vs: Vec<(i32, i32)> = vec![(5, -1), (7, -2), (9, -3)];
        let es = vec![];
        let mp = vec_pair_to_map(&vs, &es);
        assert_eq!(mp, HashMap::new())
    }

    #[test]
    fn test_vec_pair_to_map3() {
        let vs: Vec<(i32, i32)> = vec![(5, -1), (7, -2), (9, -3)];
        let es = vec![1, 3, 2];
        let mp = vec_pair_to_map(&vs, &es);
        assert_eq!(mp, HashMap::from([(3, vec![]), (2, vec![]), (1, vec![])]))
    }

    #[test]
    fn test_vec_pair_to_map4() {
        let vs: Vec<(i32, i32)> = vec![(1, 5), (7, -2), (2, 3), (7, 9)];
        let es = vec![1, 3, 2, 7];
        let mp = vec_pair_to_map(&vs, &es);
        assert_eq!(
            mp,
            HashMap::from([(1, vec![5]), (3, vec![]), (2, vec![3]), (7, vec![-2, 9])])
        )
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
