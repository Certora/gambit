use crate::invoke_command;
use std::{
    error,
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
    output_directory: PathBuf,
    basepath: Option<String>,
    allow_paths: Option<Vec<String>>,
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
