use anyhow::anyhow;
use clap_derive::Subcommand;
use rex::{
    compile, get_named_expr,
    tools::printer::{Prec, print, print_expr},
};
use std::fs::read_to_string;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "rex cli")]
#[command(version = "0.1.0", about)]
struct Args {
    /// Name of the person to greet
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Run { code: String },
}

// HACK: this is set up for bootstrapping
fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // let file = read_to_string(path)?;

    let code = match args.command {
        Commands::Run { code } => code,
    };

    let expr = get_named_expr(code)?;

    println!("{}", print(expr, Prec::LOWEST, None, false));
    Ok(())
}
