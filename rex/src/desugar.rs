use std::{cell::RefCell, collections::HashMap, fmt::Display, ops::Range};

use anyhow::bail;
use chumsky::prelude::*;
use derive_more::{Deref, DerefMut};
use functor_derive::Functor;
use rex_core::{Expr, ExprTree, SpannedExprTree};
use rex_parser::parser::{NormalSugarExpr, ResultSugarExpr, SugarExpr};

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

// TODO: We need a system to allow for partial type annotation
// TODO: We also need to keep the spans in the exprTree
pub fn desugar(expr: NormalSugarExpr, loc: Vec<usize>) -> Option<SpannedExprTree<String, String>> {
    let new_expr = match expr.0 {
        SugarExpr::Var(name) => (Expr::Var { idx: name }, loc),
        SugarExpr::App(sugar_expr, sugar_expr1) => {
            let mut loc0 = loc.clone();
            let mut loc1 = loc.clone();
            loc0.push(0);
            loc1.push(1);

            (
                Expr::App {
                    func: Box::new(desugar(*sugar_expr, loc0)?),
                    arg: Box::new(desugar(*sugar_expr1, loc1)?),
                },
                loc,
            )
        }

        SugarExpr::MultiLambda(items, sugar_expr) => items.into_iter().rev().fold(
            desugar(*sugar_expr, loc.clone())?.0,
            |acc_expr, (name, ty)| {
                let Some(ty) = ty else {
                    return (Expr::Type, loc.clone());
                };

                let Some(ty) = desugar(*ty, loc.clone()) else {
                    return (Expr::Type, loc.clone());
                };
                (
                    Expr::Lambda {
                        name,
                        param_ty: Box::new(ty),
                        body: Box::new(SpannedExprTree(acc_expr)),
                    },
                    loc.clone(),
                )
            },
        ),
        SugarExpr::MultiPi(items, sugar_expr) => items.into_iter().rev().fold(
            desugar(*sugar_expr, loc.clone())?.0,
            |acc_expr, (name, ty)| {
                let Some(ty) = desugar(*ty, loc.clone()) else {
                    return (Expr::Type, loc.clone());
                };
                (
                    Expr::Pi {
                        name: name.unwrap_or_default(),
                        param_ty: Box::new(ty),
                        ret_ty: Box::new(SpannedExprTree(acc_expr)),
                    },
                    loc.clone(),
                )
            },
        ),
        // TODO: This got messed up after adding spans
        SugarExpr::LetIn(name, ty, arg, body) => {
            let mut loc0 = loc.clone();
            let mut loc1 = loc.clone();
            let mut loc2 = loc.clone();
            loc0.push(0);
            loc1.push(1);
            loc2.push(2);

            let lambda = SpannedExprTree((
                Expr::Lambda {
                    name: name,
                    param_ty: Box::new(desugar(*ty, loc0)?),
                    body: Box::new(desugar(*body, loc1)?),
                },
                loc.clone(),
            ));

            let arg = desugar(*arg, loc2)?;

            (
                Expr::App {
                    func: Box::new(lambda),
                    arg: Box::new(arg),
                },
                loc,
            )
        }

        SugarExpr::Pipe(sugar_expr, sugar_expr1) => {
            let mut loc0 = loc.clone();
            let mut loc1 = loc.clone();
            loc0.push(0);
            loc1.push(1);

            (
                Expr::App {
                    func: Box::new(desugar(*sugar_expr1, loc0)?),
                    arg: Box::new(desugar(*sugar_expr, loc1)?),
                },
                loc,
            )
        }
        SugarExpr::Group(expr) => {
            let mut loc0 = loc.clone();
            loc0.push(0);
            desugar(*expr, loc0)?.0
        }
        SugarExpr::Type => (Expr::Type, loc),
        SugarExpr::Ann(expr, ty) => todo!("We do not support general type annotations yet"),
    };
    Some(SpannedExprTree(new_expr))
}

pub type Context = Vec<String>;

pub fn resolve(name: String, ctx: &mut Context) -> Option<usize> {
    ctx.iter().rev().position(|n| *n == name)
}

#[derive(thiserror::Error, Functor, Debug)]
pub enum ResolveError<T> {
    #[error("failed to resolve variable {0:?} at {1:?} ")]
    ResolveFailed(String, T),
}
// zipper needed
pub fn to_indices(
    expr: ExprTree<String, String>,
) -> Result<ExprTree<usize, ()>, ResolveError<Vec<usize>>> {
    fn go(
        expr: ExprTree<String, String>,
        env: &mut Vec<String>,
        loc: Vec<usize>,
    ) -> Result<ExprTree<usize, ()>, ResolveError<Vec<usize>>> {
        let expr = match expr.0 {
            Expr::Var { idx: x } => {
                if let Some(pos) = env.iter().rev().position(|y| *y == x) {
                    Expr::Var { idx: pos }
                } else {
                    return Err(ResolveError::ResolveFailed(x, loc));
                }
            }
            Expr::App { func, arg } => {
                let mut loc0 = loc.clone();
                let mut loc1 = loc.clone();
                loc0.push(0);
                loc1.push(1);
                Expr::App {
                    func: Box::new(go(*func, env, loc0)?),
                    arg: Box::new(go(*arg, env, loc1)?),
                }
            }
            Expr::Lambda {
                name,
                param_ty,
                body,
            } => {
                let mut loc0 = loc.clone();
                let mut loc1 = loc.clone();
                loc0.push(0);
                loc1.push(1);

                let param_ty = Box::new(go(*param_ty, env, loc0)?);
                env.push(name.clone());
                let res = Expr::Lambda {
                    name: (),
                    param_ty,
                    body: Box::new(go(*body, env, loc1)?),
                };
                env.pop();
                res
            }
            Expr::Pi {
                name,
                param_ty,
                ret_ty,
            } => {
                let mut loc0 = loc.clone();
                let mut loc1 = loc.clone();
                loc0.push(0);
                loc1.push(1);

                let param_ty = Box::new(go(*param_ty, env, loc0)?);
                env.push(name.clone());
                let res = Expr::Pi {
                    name: (),
                    param_ty,
                    ret_ty: Box::new(go(*ret_ty, env, loc1)?),
                };
                env.pop();
                res
            }
            Expr::Type => Expr::Type,
        };
        Ok(ExprTree(expr))
    }
    go(expr, &mut Vec::new(), Vec::new())
}
