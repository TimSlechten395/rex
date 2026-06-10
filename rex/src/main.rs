use anyhow::{Context, anyhow};
use clap_derive::Subcommand;
use rex::{
    bootstrap::compile_min_version,
    compile,
    data::expr::{Expr, ExprF, GDef, GExpr},
    eval::{normal_form, weak_head_normal_form},
    helper::find_char,
    pipeline::{
        desugar::{create_accessor, create_string},
        name_resolver::{self, to_indices},
    },
    tools::printer::{Prec, print_expr, print_named_expr},
};
use serde_json::to_string_pretty;
use std::{
    fs::{self, read_to_string},
    path::PathBuf,
    str::FromStr,
};

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
    Run {
        code: Source,
        #[arg(long, short)]
        bootstrap: bool,
    },
}

#[derive(Debug, Clone)]
enum Source {
    Inline(String),
    File(String),
}

impl FromStr for Source {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(path) = s.strip_prefix('@') {
            Ok(Source::File(path.to_string()))
        } else {
            Ok(Source::Inline(s.to_string()))
        }
    }
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // let file = read_to_string(path)?;

    match args.command {
        Commands::Run { code, bootstrap } => {
            let code = match code {
                Source::Inline(code) => Ok(code),
                Source::File(path) => {
                    read_to_string(&path).with_context(|| format!("failed to read file: {path}"))
                }
            }?;

            let (tokens, ast, expr, ty_errors) = compile(code)?;
            if bootstrap {
                let exprs: Vec<(String, Expr)> = expr
                    .0
                    .into_iter()
                    .map(|x| (x.name.0, x.val.remove_span()))
                    .collect();
                let json = serde_json::to_string(&exprs)?;
                println!("{}", json);
            } else {
                // let msg = find_char(tokens, ast, expr, 119)?;
                // println!("{}", msg);

                for GDef { name, ty: _, val } in expr.0.into_iter() {
                    // let expr = weak_head_normal_form(val.remove_span());
                    println!("{}: {:?}", name.0, print_expr(&val.remove_span()));
                    println!("---------------------------");
                }

                for (name, ty_errors) in ty_errors.into_iter() {
                    let ty_errors = ty_errors
                        .into_iter()
                        .map(|err| err.fmap(|(expr, span)| format!("{:?}", span)))
                        .collect::<Vec<_>>();
                    println!("{}: {:?}", name, ty_errors);
                    println!("---------------------------");
                }
            }

            // for res in expr.clone() {
            //     match res {
            //         Ok(e) => {
            //             let e = normal_form(e.0);
            //             println!(
            //                 "expr: {:?}",
            //                 print_expr(e, Prec::LOWEST, None, false, &mut Vec::new())
            //             )
            //         }
            //         Err(e) => println!("error: {:?}", e),
            //     }
            //     println!("-------------------------------------------------------");
            // }
        }
    };

    Ok(())
}
