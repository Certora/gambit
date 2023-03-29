use crate::invoke_command;
use crate::SolAST;
use serde_json::Value;
use std::{
    error,
    fs::File,
    path::{Path, PathBuf},
};

/// compilation constants
static TMP: &str = "tmp.sol";
static INPUT_JSON: &str = "input_json";
static BASEPATH: &str = "--base-path";
static ALLOWPATH: &str = "--allow-paths";
static DOT_JSON: &str = ".json";

/// Compilation configurations. This exists across compilations of individual
/// files
#[derive(Debug, Clone)]
struct SolcConf {
    solc: String,
    output_directory: PathBuf,
    basepath: Option<String>,
    allow_paths: Option<Vec<String>>,
    remappings: Option<Vec<String>>,
    optimize: bool,
}

impl SolcConf {
    pub fn new(solc: String, output_directory: PathBuf) -> SolcConf {
        SolcConf {
            solc: solc,
            output_directory: output_directory,
            basepath: None,
            allow_paths: None,
            remappings: None,
            optimize: true,
        }
    }

    pub fn with_basepath(&mut self, basepath: String) -> &Self {
        self.basepath = Some(basepath);
        self
    }

    pub fn with_allow_paths(&mut self, allow_paths: &mut Vec<String>) -> &Self {
        match self.allow_paths {
            None => {
                let mut paths = vec![];
                paths.append(allow_paths);
                self.allow_paths = Some(paths);
            }
            Some(mut vec) => {
                vec.append(allow_paths);
            }
        };
        &self
    }

    pub fn with_remappings(&mut self, remappings: &mut Vec<String>) -> &Self {
        match self.remappings {
            None => {
                let mut remaps = vec![];
                remaps.append(remappings);
                self.allow_paths = Some(remaps);
            }
            Some(mut vec) => {
                vec.append(remappings);
            }
        };
        &self
    }
}

/// A wrapper around solc. A separate Solc must be created for each compiled file.
struct Solc {
    conf: SolcConf,
    ast_dir: PathBuf,
}

impl Solc {
    /// Compile a solidity file to an AST
    ///
    /// This method:
    /// 1. Creates a new directory in `self.conf.output_directory` to store the
    ///    compiled AST file
    /// 2. Invokes solc with flags derived from `self.conf`
    /// 3. Copies the AST file to a JSON file in the same directory
    /// 4. Reads the JSON into a SolAST struct and returns it
    pub fn compile(&self, solidity_file: &Path) -> Result<SolAST, Box<dyn error::Error>> {
        let outdir = self.conf.output_directory;
        let (ast_dir, ast_path, json_path) = Self::make_ast_dir(solidity_file, outdir.as_path())?;
        self.invoke_compiler(solidity_file, &ast_dir);

        std::fs::copy(ast_path, &json_path)?;

        let json_f = File::open(&json_path)?;
        let ast_json: Value = serde_json::from_reader(json_f)?;
        Ok(SolAST {
            element: Some(ast_json),
            contract: None,
        })
    }

    /// Perform the actual compilation by invoking a process. This is a wrapper
    /// around `util::invoke_command`.
    fn invoke_compiler(
        &self,
        solidity_file: &Path,
        ast_dir: &Path,
    ) -> Result<(), Box<dyn error::Error>> {
        let flags = self.make_compilation_flags(solidity_file, ast_dir);
        let pretty_flags = flags
            .iter()
            .map(|x| x.to_string())
            .collect::<Vec<_>>()
            .join(" ");

        log::info!(
            "Invoking solc on {}: `{} {}`",
            solidity_file.display(),
            self.conf.solc,
            pretty_flags,
        );

        let (code, _, _) = invoke_command(&self.conf.solc, flags)?;

        match code {
            None => {
                log::error!("Solc termianted with a signal");
                return Err("Solc terminated with a signal".into());
            }
            Some(code) => {
                if code != 0 {
                    log::error!("Solidity compiler failed unexpectedly.");
                    eprintln!("Solidity compiler failed unexpectedly. For more details, try running the following command from your terminal:");
                    eprintln!("`{} {}`", &self.conf.solc, pretty_flags);
                    std::process::exit(1)
                }
            }
        };
        Ok(())
    }

    /// A helper function to create the directory where the AST (.ast) and it's
    /// json representation (.ast.json) are stored.
    ///
    /// # Arguments
    ///
    /// * `solidity_file` - Solidity file that is going to be compiled
    /// * `output_directory` - The output directory
    ///
    /// # Returns
    ///
    /// This returns a 3-tuple:
    /// * `ast_dir` - the path to the directory of the solidity AST
    /// * `ast_path` - the solidity AST file (contained inside `sol_ast_dir`)
    /// * `json_path` - the solidity AST JSON file (contained inside
    ///   `sol_ast_dir`)
    fn make_ast_dir(
        solidity_file: &Path,
        output_directory: &Path,
    ) -> Result<(PathBuf, PathBuf, PathBuf), Box<dyn error::Error>> {
        let sol_path = Path::new(solidity_file);
        let extension = sol_path.extension();
        if extension.is_none() || !extension.unwrap().eq("sol") {
            panic!("Invalid Extension: {}", solidity_file.display());
        }
        let sol_ast_dir = output_directory
            .join(INPUT_JSON.to_owned())
            .join(solidity_file);

        std::fs::create_dir_all(sol_ast_dir.parent().unwrap())?;

        let ast_fnm = Path::new(solidity_file)
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned()
            + "_json.ast";
        let ast_path = sol_ast_dir.join(&ast_fnm);
        let json_path = sol_ast_dir.join(ast_fnm + DOT_JSON);
        Ok((sol_ast_dir, ast_path, json_path))
    }

    fn make_compilation_flags(&self, solidity_file: &Path, ast_dir: &Path) -> Vec<&str> {
        let mut flags: Vec<&str> = vec![
            "--ast-compact-json",
            solidity_file.to_str().unwrap(),
            "-o", // TODO: Do we do this by default?
            ast_dir.to_str().unwrap(),
            "--overwrite",
        ];

        if let Some(basepath) = self.conf.basepath {
            flags.push(BASEPATH);
            flags.push(&basepath);
        }

        if let Some(allow_paths) = &self.conf.allow_paths {
            flags.push(ALLOWPATH);
            for r in allow_paths {
                flags.push(r);
            }
        }

        if let Some(remaps) = &self.conf.remappings {
            for r in remaps {
                flags.push(r);
            }
        }

        flags
    }
}
