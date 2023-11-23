use itertools::join;
use crate::invoke_command;
use crate::SolAST;
use serde_json::Value;
use std::{
    error,
    fs::File,
    path::{Path, PathBuf},
};

type CompilerRet = (i32, Vec<u8>, Vec<u8>);

/// This module provides a wrapper around the solc compiler, as well as several
/// helper functions. The main object of interest in this module is `Solc`.

/// compilation constants
static INPUT_JSON: &str = "input_json";
static ALLOWPATHS: &str = "--allow-paths";
static INCLUDEPATH: &str = "--include-path";
static BASEPATH: &str = "--base-path";
static OPTIMIZE: &str = "--optimize";
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
    include_path: Option<String>,
    remappings: Option<Vec<String>>,
    optimize: bool,
}

impl Solc {
    pub fn new(solc: String, output_directory: PathBuf) -> Solc {
        Solc {
            solc,
            output_directory,
            basepath: None,
            allow_paths: None,
            include_path: None,
            remappings: None,
            optimize: false,
        }
    }

    pub fn output_directory(&self) -> &Path {
        &self.output_directory
    }

    pub fn basepath(&self) -> Option<&String> {
        match &self.basepath {
            Some(bp) => Some(bp),
            None => None,
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

    pub fn with_include_path(&mut self, include_path: String) -> &Self {
        self.include_path = Some(include_path);
        self
    }

    pub fn with_remappings(&mut self, remappings: Vec<String>) -> &Self {
        self.remappings = Some(remappings);
        self
    }

    pub fn with_optimize(&mut self, optimize: bool) -> &Self {
        self.optimize = optimize;
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
        log::debug!(
            "Invoking AST compilation (--stop-after parse) on {}",
            solidity_file.display()
        );
        let outdir = &self.output_directory;
        let mk_dir_result = Self::make_ast_dir(solidity_file, outdir.as_path());
        let (ast_dir, ast_path, json_path) = match mk_dir_result {
            Ok(x) => x,
            Err(e) => {
                log::error!(
                    "Error: Failed to run make_ast_dir({}, {})\nEncountered error {}",
                    solidity_file.display(),
                    outdir.as_path().display(),
                    e
                );
                return Err(e);
            }
        };

        match self.invoke_compiler(solidity_file, &ast_dir, false) {
            Ok((code, stdout, stderr)) => {
                if code != 0 {
                    log::error!(
                        "Solidity compiler returned exit code {} on file `{}`",
                        code,
                        solidity_file.display()
                    );
                    log::error!("stdout: {}", String::from_utf8(stdout).unwrap());
                    log::error!("stderr: {}", String::from_utf8(stderr).unwrap());
                }
            }
            Err(e) => {
                log::error!(
                "Failed to compile source with invoke_compiler({}, {}, {}) \nEncountered error {}",
                solidity_file.display(),
                ast_dir.display(),
                true,
                e
            );
                return Err(e);
            }
        }
        std::fs::copy(&ast_path, &json_path)?;
        log::debug!("Wrote AST to {}", &ast_path.display());
        log::debug!("Wrote AST as JSON to {}", &json_path.display());

        let json_f = File::open(&json_path)?;
        let ast_json: Value = serde_json::from_reader(json_f)?;
        log::debug!("Deserialized JSON AST from {}", &json_path.display());
        Ok(SolAST {
            element: Some(ast_json),
        })
    }

    /// Invoke the full solidity compiler and return the exit code, stdout, and stderr
    pub fn compile(
        &self,
        solidity_file: &Path,
        outdir: &Path,
    ) -> Result<CompilerRet, Box<dyn error::Error>> {
        log::debug!("Invoking full compilation on {}", solidity_file.display());
        self.invoke_compiler(solidity_file, outdir, false)
    }

    /// Perform the actual compilation by invoking a process. This is a wrapper
    /// around `util::invoke_command`.
    ///
    /// # Return
    ///
    /// Returns the exit code, stdout, and stderr. By default we don't report
    /// errors in compilation as these can be expected (e.g., during
    /// validation). However, it is possible that compilation _should not fail_
    /// (e.g., when doing an initial compilation of an original unmutated
    /// solidity file), and getting detailed information on why such a
    /// compilation failed is important!
    fn invoke_compiler(
        &self,
        solidity_file: &Path,
        ast_dir: &Path,
        stop_after_parse: bool,
    ) -> Result<CompilerRet, Box<dyn error::Error>> {
        let flags = self.make_compilation_flags(solidity_file, ast_dir, stop_after_parse);
        let flags: Vec<&str> = flags.iter().map(|s| s as &str).collect();
        let pretty_flags = flags
            .iter()
            .map(|x| x.to_string())
            .collect::<Vec<_>>()
            .join(" ");

        log::debug!(
            "Invoking solc on {}: `{} {}`",
            solidity_file.display(),
            self.solc,
            pretty_flags,
        );

        let (code, stdout, stderr) = invoke_command(&self.solc, flags)?;

        match code {
            None => {
                // We report this as an error because something bad happened!
                log::error!("Solc terminated with a signal");
                log::error!("  stderr: {}", String::from_utf8_lossy(&stderr));
                log::error!("  stdout: {}", String::from_utf8_lossy(&stdout));
                Err("Solc terminated with a signal".into())
            }
            Some(code) => {
                // We report this as a info/debug because non-zero exit codes
                // are expected during validation. We are returning stdout and
                // stderr in case they are needed by the caller to explain an
                // unexpected compilation failure
                if code != 0 {
                    log::info!(
                        "Running solc on {} finished with non-zero code {}",
                        solidity_file.display(),
                        code
                    );
                    log::debug!("Ran `{} {}`", &self.solc, pretty_flags);
                    log::debug!("  stderr: {}", String::from_utf8_lossy(&stderr));
                    log::debug!("  stdout: {}", String::from_utf8_lossy(&stdout));
                }
                Ok((code, stdout, stderr))
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

        let input_json_dir = output_directory.join(INPUT_JSON);
        if input_json_dir.exists() {
            log::debug!("{} already exists", input_json_dir.display());
        } else {
            log::debug!("{} doesn't exist", input_json_dir.display());
        }

        let filename = PathBuf::from(solidity_file.file_name().unwrap());
        let sol_ast_dir = input_json_dir
            .join(filename)
            .parent()
            .unwrap()
            .to_path_buf();

        std::fs::create_dir_all(&sol_ast_dir)?;
        log::debug!("Created AST directory {}", input_json_dir.display());

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
            flags.push(BASEPATH.into());
            flags.push(basepath.clone());
        }

        if let Some(allow_paths) = &self.allow_paths {
            flags.push(ALLOWPATHS.into());
            let comma_separated = join(allow_paths, ",");
            flags.push(comma_separated);
            
        }

        if let Some(include_path) = &self.include_path {
            flags.push(INCLUDEPATH.into());
            flags.push(include_path.clone());
        }

        if let Some(remaps) = &self.remappings {
            for r in remaps {
                flags.push(r.clone());
            }
        }

        if self.optimize {
            flags.push(OPTIMIZE.into());
        }

        flags
    }
}
