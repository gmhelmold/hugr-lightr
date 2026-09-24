use bench_runner::{verify_spec, run, merge};
use clap::{Parser, Subcommand};

use verify_spec::VerifySpecArgs;
use run::RunArgs;
use merge::MergeArgs;

#[derive(Parser)]
#[command(name = "bench-runner", version, about = "Docker vs Lightr benchmark runner")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Validate benchmark spec YAML
    VerifySpec(VerifySpecArgs),
    /// Run benchmark chunk and emit raw JSONL evidence
    Run(RunArgs),
    /// Merge raw JSONL chunks and emit summary
    Merge(MergeArgs),
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::VerifySpec(args) => verify_spec::execute(args),
        Commands::Run(args) => run::execute(args),
        Commands::Merge(args) => merge::execute(args),
    }
}