use crate::{invoke_command, MutateParams};
use std::{
    env, error,
    path::{Path, PathBuf},
};

type CompilerRet = (i32, Vec<u8>, Vec<u8>);

/// This module provides a wrapper around the solc compiler, as well as several
/// helper functions. The main object of interest in this module is `Solc`.

/// compilation constants
static ALLOWPATH: &str = "--allow-paths";
static OPTIMIZE: &str = "--optimize";

/// Compilation configurations. This exists across compilations of individual
/// files
#[derive(Debug, Clone)]
pub struct Solc {
    /// The solc executable string
    pub solc: String,
    /// The output directory for solc to compile to (-o|--output-dir in solc)
    output_directory: PathBuf,
    /// The root of the virtual filesystem (--base-path in solc)
    basepath: Option<String>,
    /// Make additional source directory availabe to the default import callback (--include-path in solc)
    include_paths: Vec<String>,
    /// Allow a given path for imports (--allow-paths in solc)
    allow_paths: Option<Vec<String>>,
    /// Specify remappings (xyz=/path/to/xyz in solc)
    remappings: Option<Vec<String>>,
    /// Enable optimization flag (--optimize in solc)
    optimize: bool,
    /// Bypass all other flags and manually specify raw arguments passed to solc
    raw_args: Option<Vec<String>>,
}

impl Solc {
    pub fn new(solc: String, output_directory: PathBuf) -> Solc {
        Solc {
            solc,
            output_directory,
            basepath: None,
            include_paths: vec![],
            allow_paths: None,
            remappings: None,
            optimize: false,
            raw_args: None,
        }
    }

    pub fn with_vfs_roots_from_params(&mut self, params: &MutateParams) -> &Self {
        if !params.import_paths.is_empty() {
            self.with_basepath(params.import_paths.get(0).unwrap().clone());
            for path in params.import_paths[1..].iter() {
                self.with_include_path(path.clone());
            }
        }
        self.with_remappings(params.import_maps.clone());
        self
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

    pub fn with_output_directory(&mut self, output_directory: PathBuf) -> &Self {
        self.output_directory = output_directory;
        self
    }

    pub fn with_basepath(&mut self, basepath: String) -> &Self {
        self.basepath = Some(basepath);
        self
    }

    pub fn with_include_path(&mut self, include_path: String) -> &Self {
        self.include_paths.push(include_path);
        self
    }

    pub fn with_import_path(&mut self, import_path: String) -> &Self {
        if self.basepath.is_none() {
            self.basepath = Some(import_path);
        } else {
            self.include_paths.push(import_path);
        }
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

    pub fn with_optimize(&mut self, optimize: bool) -> &Self {
        self.optimize = optimize;
        self
    }

    pub fn with_raw_args(&mut self, raw_args: Vec<String>) -> &Self {
        self.raw_args = Some(raw_args);
        self
    }
}

impl Solc {
    /// Invoke the full solidity compiler and return the exit code, stdout, and stderr
    pub fn compile(&self, solidity_file: &Path) -> Result<CompilerRet, Box<dyn error::Error>> {
        log::debug!("Invoking full compilation on {}", solidity_file.display());
        self.invoke_compiler(solidity_file)
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
    fn invoke_compiler(&self, solidity_file: &Path) -> Result<CompilerRet, Box<dyn error::Error>> {
        let flags = self.make_compilation_flags(solidity_file);
        let flags: Vec<&str> = flags.iter().map(|s| s as &str).collect();
        let pretty_flags = flags
            .iter()
            .map(|x| x.to_string())
            .collect::<Vec<_>>()
            .join(" ");

        log::debug!(
            "Invoking solc on {} from {}: `{} {}`",
            solidity_file.display(),
            env::current_dir()?.display(),
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

    /// Create the compilation flags for compiling `solidity_file` in `ast_dir`
    fn make_compilation_flags(&self, solidity_file: &Path) -> Vec<String> {
        if let Some(ref flags) = self.raw_args {
            flags.clone()
        } else {
            let mut flags: Vec<String> = vec![
                solidity_file.to_str().unwrap().into(),
                "--output-dir".into(),
                self.output_directory.to_str().unwrap().into(),
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

            if self.optimize {
                flags.push(OPTIMIZE.into());
            }

            flags
        }
    }
}
