use std::path::{Path, PathBuf};

pub fn get_path_normals(path: &str) -> PathBuf {
    let path = Path::new(path);
    let comps = path.components().collect::<Vec<_>>();
    let normals: Vec<PathBuf> = comps
        .iter()
        .filter(|c| matches!(c, std::path::Component::Normal(_)))
        .map(|c| Path::new(c).to_path_buf())
        .collect();
    let mut root = normals[0].clone();
    for item in normals.iter().skip(1) {
        root.push(&item);
    }
    root.to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_convert_path1() {
        let path1 = "../../../gambit/out/foo/TenPower.sol0.sol";
        let res1 = get_path_normals(path1);
        assert_eq!(
            res1,
            Path::new("gambit/out/foo/TenPower.sol0.sol").to_path_buf()
        );
    }

    #[test]
    fn test_convert_path2() {
        let path2 = "gambit/out/foo/TenPower.sol0.sol";
        let res2 = get_path_normals(path2);
        assert_eq!(
            res2,
            Path::new("gambit/out/foo/TenPower.sol0.sol").to_path_buf()
        );
    }

    #[test]
    fn test_convert_path3() {
        let path3 = "/gambit/out/foo/TenPower";
        let res3 = get_path_normals(path3);
        assert_eq!(res3, Path::new("gambit/out/foo/TenPower").to_path_buf());
    }
}
