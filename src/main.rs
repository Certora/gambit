use std::time::Instant;

use clap::Parser;
use gambit::{Command, MutantFilter, MutantWriter, MutateParams, Mutator, RandomDownSampleFilter};

/// Entry point
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = env_logger::builder().try_init();
    match Command::parse() {
        Command::Mutate(params) => {
            mutate(params)?;
        }
    }
    Ok(())
}

/// Execute the `mutate` command
fn mutate(params: MutateParams) -> Result<(), Box<dyn std::error::Error>> {
    let start = Instant::now();
    let mut mutator = Mutator::from(&params);
    let mutants = mutator.mutate()?.clone();

    let mutants = if let Some(num_mutants) = params.num_mutants {
        // TODO: make num_mutants an Option
        let filter = RandomDownSampleFilter::new(params.seed, !params.skip_validate);
        filter.filter_mutants(&mutator, num_mutants)?
    } else {
        if params.skip_validate {
            mutants
        } else {
            mutator.get_valid_mutants(&mutants)
        }
    };

    MutantWriter::new(
        params.outdir,
        params.log_mutants,
        params.export_mutants,
        params.overwrite,
    )
    .write_mutants(&mutants)?;

    let t = start.elapsed().as_secs_f64();
    println!("Generated {} mutants in {:.2} seconds", &mutants.len(), t);
    log::info!("Generated {} mutants in {}", &mutants.len(), t);
    Ok(())
}
