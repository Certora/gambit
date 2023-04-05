use crate::invoke_command;
use crate::SolAST;
use serde_json::Value;
use std::{
    error,
    fs::File,
    path::{Path, PathBuf},
};

/// This module provides a wrapper around the solc compiler, as well as several
/// helper functions. The main object of interest in this module is `Solc`.

/// compilation constants
static INPUT_JSON: &str = "input_json";
static ALLOWPATH: &str = "--allow-paths";
static DOT_JSON: &str = ".json";

/// Compilation configurations. This exists across compilations of individual
/// files
#[derive(Debug, Clone)]
pub struct Solc {
    /// The solc executable string
    pub solc: String,
    output_directory: PathBuf,
    basepath: Option<String>,
    allow_paths: Option<Vec<String>>,
    remappings: Option<Vec<String>>,
}

impl Solc {
    pub fn new(solc: String, output_directory: PathBuf) -> Solc {
        Solc {
            solc: solc,
            output_directory: output_directory,
            basepath: None,
            allow_paths: None,
            remappings: None,
        }
    }

    pub fn with_basepath(&mut self, basepath: String) -> &Self {
        self.basepath = Some(basepath);
        self
    }

    pub fn with_allow_paths(&mut self, allow_paths: Vec<String>) -> &Self {
        self.allow_paths = Some(allow_paths);
        self
    }

    pub fn with_remappings(&mut self, remappings: Vec<String>) -> &Self {
        self.remappings = Some(remappings);
        self
    }
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
    pub fn compile_ast(&self, solidity_file: &Path) -> Result<SolAST, Box<dyn error::Error>> {
        let outdir = &self.output_directory;
        let (ast_dir, ast_path, json_path) = Self::make_ast_dir(solidity_file, outdir.as_path())?;
        self.invoke_compiler(solidity_file, &ast_dir, true)?;

        std::fs::copy(ast_path, &json_path)?;

        let json_f = File::open(&json_path)?;
        let ast_json: Value = serde_json::from_reader(json_f)?;
        Ok(SolAST {
            element: Some(ast_json),
        })
    }

    /// Invoke the full solidity compiler and return the exit code
    pub fn compile_full(
        &self,
        solidity_file: &Path,
        outdir: &Path,
    ) -> Result<i32, Box<dyn error::Error>> {
        self.invoke_compiler(solidity_file, outdir, false)
    }

    /// Perform the actual compilation by invoking a process. This is a wrapper
    /// around `util::invoke_command`.
    fn invoke_compiler(
        &self,
        solidity_file: &Path,
        ast_dir: &Path,
        stop_after_parse: bool,
    ) -> Result<i32, Box<dyn error::Error>> {
        let flags = self.make_compilation_flags(solidity_file, ast_dir, stop_after_parse);
        let flags: Vec<&str> = flags.iter().map(|s| s as &str).collect();
        let pretty_flags = flags
            .iter()
            .map(|x| x.to_string())
            .collect::<Vec<_>>()
            .join(" ");

        log::info!(
            "Invoking solc on {}: `{} {}`",
            solidity_file.display(),
            self.solc,
            pretty_flags,
        );

        let (code, stdout, stderr) = invoke_command(&self.solc, flags)?;

        match code {
            None => {
                eprintln!("Solc terminated with a singal");
                eprintln!("  stderr: {}", String::from_utf8_lossy(&stderr));
                eprintln!("  stdout: {}", String::from_utf8_lossy(&stdout));
                log::error!("Solc terminated with a signal");
                Err("Solc terminated with a signal".into())
            }
            Some(code) => {
                if code != 0 {
                    log::error!("Solidity compiler failed unexpectedly.");
                    eprintln!("  stderr: {}", String::from_utf8_lossy(&stderr));
                    eprintln!("  stdout: {}", String::from_utf8_lossy(&stdout));
                    eprintln!("Solidity compiler failed unexpectedly. For more details, try running the following command from your terminal:");
                    eprintln!("`{} {}`", &self.solc, pretty_flags);
                }
                Ok(code)
            }
        }
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
        let extension = solidity_file.extension();
        if extension.is_none() || !extension.unwrap().eq("sol") {
            panic!("Invalid Extension: {}", solidity_file.display());
        }
        let filename = PathBuf::from(solidity_file.file_name().unwrap());
        let sol_ast_dir = output_directory.join(INPUT_JSON.to_owned()).join(filename);

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

    /// Create the compilation flags for compiling `solidity_file` in `ast_dir`
    ///
    /// TODO: I'm currently cloning `String`s because of lifetime issues, but I'd
    /// like to convert this back to `&str`s.
    fn make_compilation_flags(
        &self,
        solidity_file: &Path,
        ast_dir: &Path,
        stop_after_parse: bool,
    ) -> Vec<String> {
        let mut flags: Vec<String> = vec![
            "--ast-compact-json".into(),
            solidity_file.to_str().unwrap().into(),
            "--output-dir".into(),
            ast_dir.to_str().unwrap().into(),
            "--overwrite".into(),
        ];
        if stop_after_parse {
            flags.push("--stop-after".into());
            flags.push("parsing".into());
        }

        if let Some(basepath) = &self.basepath {
            flags.push("--base-path".into());
            flags.push(basepath.clone());
        }

        if let Some(allow_paths) = &self.allow_paths {
            flags.push(ALLOWPATH.into());
            for r in allow_paths {
                flags.push(r.clone());
            }
        }

        if let Some(remaps) = &self.remappings {
            for r in remaps {
                flags.push(r.clone());
            }
        }

        flags
    }
}
