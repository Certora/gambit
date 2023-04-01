mod ast;
pub use ast::*;

mod cli;
pub use cli::*;

mod compile;
pub use compile::*;

mod filter;
pub use filter::*;

mod mutation;
pub use mutation::*;

mod mutant_writer;
pub use mutant_writer::*;

mod mutator;
pub use mutator::*;

mod source;
pub use source::*;
mod util;
pub use util::*;

#[derive(Debug, Clone)]
pub struct MutantGenerator {
    /// Params for controlling the mutants.
    pub params: MutateParams,
}
