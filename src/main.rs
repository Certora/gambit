use clap::Parser;
use serde::{Deserialize, Serialize};

/**
 * Workflow:
 * Let's mainly focus on the mutation
 * generation part for now.
 * This tool should take as input, a solidity file,
 * then compile it to generate it's AST and do various mutations of it.
 * All the mutated files should be in some directory that the user will
 * pass from the commandline.
 */ 

//  #[derive(Debug, Clone)]
// pub struct MutantGenerator {
//     m
// } 

#[derive(Parser, Deserialize, Serialize)]
#[clap(rename_all = "kebab-case")]
pub struct MutationParams {
    // directory to store all mutants
    #[clap(long, default_value = "out")]
    pub outdir: String,
    // file(s) to mutate
    #[clap(long, required=true, multiple=true)]
    pub filename: String,   
}

#[derive(Parser)]
#[clap(rename_all = "kebab-case")]
pub enum Command {
    Mutate(MutationParams)
    // Maybe we want to do other things in the future? 
}

fn main() {
    let _ = env_logger::builder().try_init();
    match Command::parse() {
        Command::Mutate(params) => {
            println!("{}", params.filename)
        },
    }
}
