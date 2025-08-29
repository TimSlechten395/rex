use std::{cell::RefCell, collections::HashMap, fmt::Display, ops::Range};

use crate::{SugarExpr, Var};
use chumsky::prelude::*;
use derive_more::{Deref, DerefMut};
use rex_core::Expr;
use rex_parser::parser::SpannedSugarExpr;

use crate::Token;

// We could implement it like this so we can add metadata or even to share structure between
// expr and sugarexpr but the rust compiler has a hard time resolving types in the parser already
// and this pushes it too far
// #[derive(Debug, Clone, PartialEq, Eq, Hash)]
// pub enum ExprCore<T> {
//     Var(Var),
//     App(Box<T>, Box<T>),
//     // The first arg is always a var
//     // This will become a problem with type annotations since it needs to be an expression as well
//     Lambda(Var, Box<T>),
//     // iam hoping Fn and Pi can be the same
//     Pi(Var, Box<T>, Box<T>),
//     Universe(usize),
//     // Builtin for now
//     Int(i64),
//     Bool(bool),
//     Atom(String),
//     Sigma(Var, Box<T>, Box<T>),
//     // does this need to be builtin?
//     Builtin(BuiltinOp),
// }
//

#[derive(Debug, Deref, DerefMut, Clone, PartialEq, Eq)]
pub struct ExprTree(pub Expr<Box<ExprTree>>);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BuiltinOp {
    Car,  // first eliminator of pair
    Cdr,  // second eliminator of pair
    Cons, // pair constructor
    Pair, // pair type constructor
    Eq,   // equality constructor
    Claim,
    Define,
    Type,
    Just,
}

impl Display for BuiltinOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // BuiltinOp::Arrow => write!(f, "->"),
            BuiltinOp::Car => write!(f, "car"),
            BuiltinOp::Cdr => write!(f, "cdr"),
            BuiltinOp::Eq => write!(f, "="),
            BuiltinOp::Cons => write!(f, "cons"),
            BuiltinOp::Pair => write!(f, "Pair"),
            BuiltinOp::Claim => write!(f, "claim"),
            BuiltinOp::Define => write!(f, "define"),
            BuiltinOp::Type => write!(f, "type"),
            BuiltinOp::Just => write!(f, "just"),
        }
    }
}

impl Display for ExprTree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &**self {
            Expr::Var { idx: var_id } => write!(f, "{}", var_id),
            // This is not clear how it should display
            Expr::App { func, arg } => {
                if let Expr::App { .. } = &***arg {
                    write!(f, "{} ({})", &**func, &**arg)
                } else {
                    write!(f, "{} {}", &**func, &**arg)
                }
            }
            Expr::Lambda { param_ty, body } => write!(f, "lambda {} {}", param_ty, body),
            Expr::Pi {
                param_ty: _,
                ret_ty,
            } => write!(f, "Pi (?) {}", ret_ty),
            Expr::Type => write!(f, "Type"),
        }
    }
}

pub type Context = Vec<String>;

pub fn resolve(name: String, ctx: &mut Context) -> Option<usize> {
    ctx.iter().rev().position(|n| *n == name)
}

// TODO: We need a system to allow for partial type annotation
// TODO: We also need to keep the spans in the exprTree
pub fn desugar(expr: SpannedSugarExpr, ctx: &mut Context) -> ExprTree {
    let new_expr = match expr.0.0 {
        SugarExpr::Var(name) => {
            let idx = resolve(name, ctx);
            if let Some(idx) = idx {
                Expr::Var { idx }
            } else {
                panic!("Unbound variable {:?}", idx)
            }
        }
        SugarExpr::App(sugar_expr, sugar_expr1) => Expr::App {
            func: Box::new(desugar(*sugar_expr, ctx)),
            arg: Box::new(desugar(*sugar_expr1, ctx)),
        },

        SugarExpr::Lambda(param, param_ty, body) => {
            let param_ty = Box::new(desugar(*param_ty, ctx));

            ctx.push(param);
            let body = Box::new(desugar(*body, ctx));
            ctx.pop();
            Expr::Lambda { param_ty, body }
        }
        SugarExpr::Pi(param, param_ty, ret_ty) => {
            let param_ty = Box::new(desugar(*param_ty, ctx));
            ctx.push(param);
            let ret_ty = Box::new(desugar(*ret_ty, ctx));
            ctx.pop();
            Expr::Pi { param_ty, ret_ty }
        }

        //
        SugarExpr::Sigma(var, ty, sugar_expr1) => {
            todo!();
        }

        SugarExpr::Ann(expr, ty) => todo!("We do not support general type annotations yet"),
        // SugarExpr::MultiLambda(items, sugar_expr) => {
        //     items
        //         .into_iter()
        //         .rev()
        //         .fold(desugar(*sugar_expr, context), |acc_expr, (name, ty)| {
        //             desugar(SugarExpr::Lambda(name, ty, Box::new(acc_expr)), context)
        //         })
        // }
        //
        // SugarExpr::MultiPi(items, sugar_expr) => {
        //     items
        //         .into_iter()
        //         .rev()
        //         .fold(desugar(*sugar_expr, context), |acc_expr, param| {
        //             if let super::Var::Ident(name) = param.0 {
        //                 bind(name, context)
        //             }
        //             Expr::Pi(param, Box::new(acc_expr))
        //         })
        // }
        // SugarExpr::MultiSigma(items, sugar_expr) => {
        //     items
        //         .into_iter()
        //         .rev()
        //         .fold(desugar(*sugar_expr, context), |acc_expr, param| {
        //             if let super::Var::Ident(name) = param.0 {
        //                 bind(name, context)
        //             }
        //             Expr::Sigma(Box::new(acc_expr))
        //         })
        // }

        // This is desugar by creating a lambda and instantly applying it.
        // This is done so we do not lose graph information.

        // let x: Nat = 3 in ....
        // TODO: This got messed up after adding spans
        SugarExpr::LetIn(name, ty, arg, body) => {
            let lambda = Box::new(ExprTree(Expr::Lambda {
                param_ty: Box::new(desugar(*ty, ctx)),
                body: Box::new(desugar(*body, ctx)),
            }));

            let arg = Box::new(desugar(*arg, ctx));

            Expr::App { func: lambda, arg }
        }
        SugarExpr::Loop(sugar_expr) => todo!(),

        SugarExpr::Pipe(sugar_expr, sugar_expr1) => Expr::App {
            func: Box::new(desugar(*sugar_expr1, ctx)),
            arg: Box::new(desugar(*sugar_expr, ctx)),
        },
        _ => todo!(),
    };
    ExprTree(new_expr)
}
