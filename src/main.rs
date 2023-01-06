use std::{io, time::Instant};

use clap::Parser;
use gambit::{Command, MutantGenerator};

/// Entry point
fn main() -> io::Result<()> {
    let start = Instant::now();
    let _ = env_logger::builder().try_init();
    match Command::parse() {
        Command::Mutate(params) => {
            let mut mutant_gen = MutantGenerator::new(params);
            mutant_gen.run()?;
        }
    }
    log::info!("Running time: {}", start.elapsed().as_secs_f64());
    Ok(())
}
