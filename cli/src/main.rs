use clap::{arg, value_parser, Command};
use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;
use std::fs::File;
use std::io::{stdout, Write};
use std::ffi::OsStr;

use commands::{precompute_half_squares, precompute_inv_lagrange_prod};

#[derive(Debug, Parser)]
#[command(name = "protostar-works-cli")]
#[command(about = "Development cli for protostar-works crate.", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands
}

#[derive(Debug, Subcommand)]
enum Commands {
    #[command(arg_required_else_help = true)]
    Precompute(PrecomputeArgs)
}

#[derive(Debug, Args)]
#[command(about = "Create precomputed tables.", long_about = "Create precomputed tables.\nWe use precomputed tables to optimize some computations.\nWe've decided to har code these values into library to reduce IDE load from macro expansion.")]
struct PrecomputeArgs {
    #[command(subcommand)]
    command: PrecomputeCommand
}

#[derive(Debug, Subcommand)]
enum PrecomputeCommand {
    InvLagrangeProd(PrecomputeInvLagrangeProdArgs),
    HalfSquares(PrecomputeHalfSquaresArgs),
}

#[derive(Debug, Args)]
struct PrecomputeInvLagrangeProdArgs {
    #[arg(

        default_value = "./src/utils/inv_lagrange_prod.rs",
        help = "output file; '-' for stdout.",
    )]
    output: PathBuf,
    #[arg(
        short, long,
        default_value = "30",
        help = "limit",
    )]
    limit: u64,
}

#[derive(Debug, Args)]
struct PrecomputeHalfSquaresArgs {
    #[arg(
        default_value = "./src/utils/half_squares.rs",
        help = "output file; '-' for stdout.",
    )]
    output: PathBuf,
    #[arg(
        short, long,
        default_value = "50",
        help = "limit",
    )]
    limit: u64,
}


fn make_output(path: &PathBuf) -> Box<dyn Write> {
    if path.as_os_str() == OsStr::new("-") {
        Box::new(stdout())
    } else {
        Box::new(File::create(path).expect("Unable to create file"))
    }
}


fn main() {
    let matches = Cli::parse();
    match &matches.command {
        Commands::Precompute(precompute_args) => match &precompute_args.command {
            PrecomputeCommand::InvLagrangeProd(inv_lagrange_prod_args) => {
                let mut out = make_output(&inv_lagrange_prod_args.output);
                precompute_inv_lagrange_prod(&mut out, inv_lagrange_prod_args.limit)
            },
            PrecomputeCommand::HalfSquares(half_squares_args) => {
                let mut out = make_output(&half_squares_args.output);
                precompute_half_squares(&mut out, half_squares_args.limit)
            }
        }
    }
}

