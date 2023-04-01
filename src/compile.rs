use crate::invoke_command;
use crate::SolAST;
use serde_json::Value;
use std::{
    error,
    fs::File,
    path::{Path, PathBuf},
};

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
    pub fn compile(&self, solidity_file: &Path) -> Result<SolAST, Box<dyn error::Error>> {
        let outdir = &self.output_directory;
        let (ast_dir, ast_path, json_path) = Self::make_ast_dir(solidity_file, outdir.as_path())?;
        self.invoke_compiler(solidity_file, &ast_dir)?;

        std::fs::copy(ast_path, &json_path)?;

        let json_f = File::open(&json_path)?;
        let ast_json: Value = serde_json::from_reader(json_f)?;
        Ok(SolAST {
            element: Some(ast_json),
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

        let (code, _, _) = invoke_command(&self.solc, flags)?;

        match code {
            None => {
                log::error!("Solc termianted with a signal");
                return Err("Solc terminated with a signal".into());
            }
            Some(code) => {
                if code != 0 {
                    log::error!("Solidity compiler failed unexpectedly.");
                    eprintln!("Solidity compiler failed unexpectedly. For more details, try running the following command from your terminal:");
                    eprintln!("`{} {}`", &self.solc, pretty_flags);
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

    /// Create the compilation flags for compiling `solidity_file` in `ast_dir`
    ///
    /// TODO: I'm currently cloning `String`s because of lifetime issues, but I'd
    /// like to convert this back to `&str`s.
    fn make_compilation_flags(&self, solidity_file: &Path, ast_dir: &Path) -> Vec<String> {
        let mut flags: Vec<String> = vec![
            "--ast-compact-json".into(),
            "--stop-after".into(),
            "parsing".into(),
            solidity_file.to_str().unwrap().into(),
            "--output-dir".into(), // TODO: Do we do this by default?
            ast_dir.to_str().unwrap().into(),
            "--overwrite".into(),
        ];

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
