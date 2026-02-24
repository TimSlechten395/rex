use anyhow::{Context, anyhow};
use clap_derive::Subcommand;
use rex::{
    bootstrap::compile_min_version,
    compile,
    data::expr::{Expr, ExprF, GExpr},
    eval::{normal_form, weak_head_normal_form},
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

            //
            // let expr = code.map(|c| {
            //     c.and_then(|c| match c.chars().nth(0).unwrap() {
            //         '~' => {
            //             let s = c.strip_prefix('~').context("expected '~' prefix")?;
            //             let (start, end) = s.split_once(':').context("expected 'N:N' format")?;
            //             Ok((
            //                 to_indices(create_accessor(
            //                     start.parse().context("invalid start number")?,
            //                     end.parse().context("invalid end number")?,
            //                 ))
            //                 .context("failed name resolving")?,
            //                 Vec::new(),
            //             ))
            //         }
            //         _ => compile(c),
            //     })
            // });
            //

            let exprs = compile(code)?;
            if bootstrap {
                let exprs: Vec<_> = exprs.0.into_iter().map(|x| (x.0, x.1.ok())).collect();
                let json = serde_json::to_string(&exprs)?;
                println!("{}", json);
            } else {
                for (name, expr) in exprs.0.into_iter() {
                    let expr = normal_form(expr?);
                    println!("{}: {:?}", name, print_expr(expr));
                    println!("---------------------------");
                }

                for (name, ty_errors) in exprs.1.into_iter() {
                    let ty_errors = ty_errors
                        .into_iter()
                        .map(|err| {
                            err.fmap(|(expr, span)| format!("{} @ {:?}", print_expr(expr), span))
                        })
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

    // let expr = expr
    //     .filter_map(|x| x.ok().map(|x| x.0))
    //     .fold(None, |acc, item| {
    //         Some(match acc {
    //             None => item,
    //             Some(left) => GExpr(ExprF::App {
    //                 func: Box::new(left),
    //                 arg: Box::new(item),
    //             }),
    //         })
    //     });
    //
    // if let Some(e) = expr {
    // let e = weak_head_normal_form(e);
    //     println!(
    //         "combined expr: {:?}",
    //         print_expr(e, Prec::LOWEST, None, false, &mut Vec::new())
    //     );
    // }

    Ok(())
}
