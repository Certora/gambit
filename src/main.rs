use std::io;

use clap::Parser;
use gambit::{Command, MutantGenerator};

/// Entry point
fn main() -> io::Result<()> {
    let _ = env_logger::builder().try_init();
    match Command::parse() {
        Command::Mutate(params) => {
            let mut mutant_gen = MutantGenerator::new(params);
            mutant_gen.run()?;
        }
    }
    Ok(())
}
