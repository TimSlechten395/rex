#![allow(clippy::uninlined_format_args)]

use std::{
    collections::HashMap,
    error::Error,
    fs::{self, read_to_string},
    path::Path,
};

use rex::{
    BuiltinOp, Context, ExprTree, SugarExpr, Token, Var, desugar, lexer, normalize, parser, repl,
    r#type::TypeContext,
};

use rustyline::{DefaultEditor, error::ReadlineError};

fn main() -> Result<(), Box<dyn Error>> {
    let mut rl = DefaultEditor::new()?;
    println!("Welcome to the best expression repl ever!");
    if rl.load_history("history.txt").is_err() {
        // println!("No previous history")
    }

    loop {
        let readline = rl.readline(">> ");

        match readline {
            Ok(line) => {
                rl.add_history_entry(line.as_str())?;
                // println!("Line: {}", line);
                //

                let lxr = lexer();
                let tokens = lxr.parse(&line).unwrap().map(|(expr, _span)| expr);
                println!("found tokens: {:?}", tokens);

                let parser = parser();

                // parse defs
                let mut defs = HashMap::<String, (SugarExpr, SugarExpr)>::new();
                let defs_string = read_to_string("defs.txt")?;

                let defs_lines = defs_string
                    .split("let ")
                    .filter(|s| !s.trim().is_empty())
                    .skip(1)
                    .map(|s| format!("let {}", s.trim()));
                // This is proof the lifetime system does not work very well

                for (mut i, def) in defs_lines.enumerate() {
                    i += 1;
                    let lxr = lexer();
                    let def_tokens = lxr
                        .parse(&def)
                        .into_result()
                        .unwrap_or_else(|e| panic!("Failed to lex def {i}: {e:?}"))
                        // remove the span for now
                        .map(|pair| pair.0);

                    let def_parser = repl::get_definition();
                    let ((ident, ty), expr) = def_parser
                        .parse(&def_tokens)
                        .into_result()
                        .unwrap_or_else(|e| {
                            panic!("Failed to parse def {i}: {e:?}\n Tokens are: {def_tokens:?}")
                        });

                    defs.insert(ident.clone(), (ty, expr));
                }

                // println!("\n--- Parsing: \"{}\" ---", input);

                let mut ctx = Context::new();
                match parser.parse(&tokens).into_result() {
                    Ok(ast) => {
                        println!("parsed AST: {:?}", &ast);

                        println!();

                        let expr_with_defs =
                            defs.into_iter()
                                .fold(ast, |ast, (def_name, (def_ty, def_expr))| {
                                    SugarExpr::LetIn(
                                        def_name,
                                        Box::new(def_ty),
                                        Box::new(def_expr),
                                        Box::new(ast),
                                    )
                                });
                        let mut expr = desugar(expr_with_defs, &mut ctx);
                        println!("converted to expr: \n {:#?}", &expr);
                        println!("reducing expression with definitions");
                        println!();

                        let mut ty_ctx = TypeContext::default();
                        // match infer_type(&expr, &mut ty_ctx) {
                        //     Ok(inferred_type) => {
                        //         let mut norm_ty = inferred_type.clone();
                        //         normalize(&mut norm_ty);
                        //         println!("Type checking successfull");
                        //         println!("The program has the type: {:?}", norm_ty);
                        //     }
                        //     Err(e) => {
                        //         eprintln!("Type checking failed!");
                        //         eprintln!("Error: {:?}", e);
                        //     }
                        // }

                        println!("\n Evaluating the program (if it has a value)...");
                        let mut evaluated_term = expr.clone();
                        normalize(&mut evaluated_term);
                        println!("Evaluation result: {:?}", evaluated_term);
                    }
                    Err(errors) => {
                        println!("Parsing Errors:");
                        for e in errors {
                            println!("{:?}", e);
                        }
                    }
                }
            }
            Err(ReadlineError::Interrupted) => break,
            Err(ReadlineError::Eof) => break,
            Err(err) => {
                println!("Error: {err:?}");
                break;
            }
        }
    }
    rl.save_history("history.txt")?;
    Ok(())

    // for (input, expected_output) in test_cases {
    //     let lexer = lexer();
    //     let tokens = lexer.parse(input).unwrap();
    //     let parser = parser();
    //     println!("\n--- Parsing: \"{}\" ---", input);
    //     match parser.parse(&tokens).into_result() {
    //         Ok(ast) => {
    //             // println!("Parsed AST: {:?}", ast);
    //             if let Some(expected) = expected_output {
    //                 if ast != expected {
    //                     println!("Test FAILED: expected {expected:?}, but got {ast:?}");
    //                     println!("The tokens were: {tokens:?}");
    //                 } else {
    //                     println!("Test PASSED: Output matches expected.");
    //                 }
    //             } else {
    //                 println!("Test FAILED: Expected an error, but parsed successfully.");
    //             }
    //         }
    //         Err(errors) => {
    //             println!("Parsing Errors:");
    //             for e in errors {
    //                 println!("  {:?}", e);
    //             }
    //             if expected_output.is_some() {
    //                 println!("Test FAILED: Expected successful parse, but got errors.");
    //             } else {
    //                 println!("Test PASSED: Expected errors, and got them.");
    //             }
    //         }
    //     }
    // }
}
