use std::time::Instant;

use clap::Parser;
use gambit::{Command, MutantFilter, MutantWriter, Mutator, RandomDownSampleFilter};

/// Entry point
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let start = Instant::now();
    let _ = env_logger::builder().try_init();
    match Command::parse() {
        Command::Mutate(params) => {
            let mut mutator = Mutator::from(&params);
            mutator.mutate()?;

            let mutants = if let Some(num_mutants) = params.num_mutants {
                // TODO: make num_mutants an Option
                let filter = RandomDownSampleFilter::new(params.seed, true);
                filter.filter_mutants(&mutator.mutants, num_mutants)?
            } else {
                mutator.mutants
            };

            let writer =
                MutantWriter::new(params.outdir, params.log_mutants, params.export_mutants);

            writer.write_mutants(&mutants)?;

            for mutant in mutants.iter() {
                println!("{}", mutant);
            }
            let t = start.elapsed().as_secs_f64();
            println!("Generated {} mutants in {:.2} seconds", &mutants.len(), t);
            log::info!("Generated {} mutants in {}", &mutants.len(), t);
        }
    }
    Ok(())
}
