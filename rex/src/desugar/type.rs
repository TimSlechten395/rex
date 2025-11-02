use std::fmt::Display;
use std::fmt::Formatter;
use std::ops::Range;

use anyhow::anyhow;
use anyhow::bail;
use chumsky::{container::Container, input::ValueInput, prelude::*};
use either::Either::{self, Right};
use functor_derive::Functor;
use num_bigint::BigUint;
use std::fmt;
use thiserror::Error;

use crate::lexer::AbsoluteIndent;
use crate::lexer::Token;

// Later all idents will become syntactic sugar for indices

// This could have a variant Common(Expr<SugarExpr>). But we need lambda, pi and var seperate
// anyway
// TODO: Is this even worth it to have?
#[derive(Debug, Clone, PartialEq, Functor)]
pub enum SugarExpr<T> {
    Var(String),
    App(Vec<T>), // Function application
    Type,        // Type of all types
    Unit,
    Lit(LitKind),
    Dot(Vec<T>),

    Ann(Vec<T>),
    Binding(Vec<T>),

    Group(Box<T>),

    // first param is types of module dependencies second is the items in the module
    // to be converted to product type
    // pub/priv is done with an extra module that just takes the full module and returns a smaller
    // module
    // bool is temporary self dep indicator
    Module(Vec<T>, Vec<T>, bool),

    // Multi-argument Lambda (sugar for nested single-arg lambdas)
    // Example: `(lambda (x:T) (y:U) body)`
    // MultiLambda(Vec<(String, Option<T>)>, T),
    Lambda(Vec<T>),

    // Multi-argument Pi Type (sugar for nested single-arg Pi types)
    // Example: `(Pi (x:T y:U) return_type)`
    Pi(Vec<T>),
    Tuple(Vec<T>),
    Sigma(Vec<T>),

    // Let binding sugar: `let name : type = value in body`
    // LetIn(String, T, T, T),
    // Pipe operator
    Pipe(Vec<T>),
}

impl<T: Display> Display for SugarExpr<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        use SugarExpr::*;

        match self {
            Var(name) => write!(f, "{name}"),
            Type => write!(f, "Type"),
            Unit => write!(f, "()"),
            Lit(lit) => write!(f, "{lit}"),

            App(exprs) => join_fmt(f, exprs, " "),
            Dot(exprs) => join_fmt(f, exprs, "."),
            Ann(exprs) => join_fmt(f, exprs, " : "),
            Binding(exprs) => join_fmt(f, exprs, " := "),
            Tuple(exprs) => join_fmt(f, exprs, ", "),
            Sigma(exprs) => join_fmt(f, exprs, "; "),
            Pi(exprs) => join_fmt(f, exprs, " -> "),
            Lambda(exprs) => join_fmt(f, exprs, " => "),
            Pipe(exprs) => join_fmt(f, exprs, " |> "),
            Group(expr) => write!(f, "({expr})"),

            Module(deps, items, selfdep) => {
                write!(f, "module(deps=[")?;
                join_fmt(f, deps, "\n")?;
                write!(f, "], items=[\n def ")?;
                join_fmt(f, items, "\n def ")?;
                write!(f, "], selfdep={})", selfdep)
            }
        }
    }
}

/// Helper: write Vec<T> to Formatter directly with separator
fn join_fmt<T: Display>(f: &mut fmt::Formatter<'_>, v: &[T], sep: &str) -> fmt::Result {
    let mut first = true;
    for item in v {
        if !first {
            write!(f, "{sep}")?;
        }
        write!(f, "{item}")?;
        first = false;
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq)]
pub enum LitKind {
    String(String),
    Number(BigUint),
}

impl Display for LitKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        use LitKind::*;
        match self {
            String(s) => write!(f, "\",{s}\""),
            Number(i) => write!(f, "{i}"),
        }
    }
}

impl<T: Clone> SugarExpr<T> {
    pub fn fold<U>(&self, init: U, f: impl Fn(U, T) -> U + Clone) -> U {
        match &self {
            SugarExpr::Unit | SugarExpr::Var(_) | SugarExpr::Type | SugarExpr::Lit(_) => init,
            SugarExpr::Group(a) => f(init, *a.clone()),
            SugarExpr::App(items)
            | SugarExpr::Dot(items)
            | SugarExpr::Lambda(items)
            | SugarExpr::Pi(items)
            | SugarExpr::Binding(items)
            | SugarExpr::Ann(items)
            | SugarExpr::Tuple(items)
            | SugarExpr::Sigma(items)
            | SugarExpr::Pipe(items) => items.clone().into_iter().fold(init, f.clone()),

            SugarExpr::Module(items1, items2, ..) => items2
                .clone()
                .into_iter()
                .fold(items1.clone().into_iter().fold(init, f.clone()), f.clone()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Functor, Error)]
pub enum SugarExprError<T> {
    #[error("Invalid expression: found tokens {0:?}")]
    InvalidExpr(Vec<Token<AbsoluteIndent>>),

    #[error("unexpected error")]
    Other(Box<T>),

    #[error("Invalid expression: found tokens {0:?}")]
    InvalidToken(Token<AbsoluteIndent>),
}

impl<T: Clone> SugarExprError<T> {
    pub fn fold<U>(&self, init: U, f: impl Fn(U, T) -> U + Clone) -> U {
        match &self {
            SugarExprError::InvalidExpr(_) | SugarExprError::InvalidToken(_) => init,
            SugarExprError::Other(a) => f(init, *a.clone()),
        }
    }
}

pub fn fold<T: Clone, U>(
    expr: Result<SugarExpr<T>, SugarExprError<T>>,
    init: U,
    f: impl Fn(U, T) -> U + Clone,
) -> U {
    match expr {
        Ok(r) => r.fold(init, f),
        Err(e) => e.fold(init, f),
    }
}

pub fn fmap<T, R>(
    expr: Result<SugarExpr<T>, SugarExprError<T>>,
    f: impl Fn(T) -> R,
) -> Result<SugarExpr<R>, SugarExprError<R>> {
    match expr {
        Ok(r) => Ok(r.fmap(f)),
        Err(e) => Err(e.fmap(f)),
    }
}

#[derive(Debug, Clone)]
pub struct NormalSugarExpr(pub SugarExpr<NormalSugarExpr>);

impl Display for NormalSugarExpr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // Delegate to the inner SugarExpr's Display implementation
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone)]
pub struct SpannedNormalSugarExpr(pub Spanned<SugarExpr<NormalSugarExpr>>);

impl NormalSugarExpr {
    pub fn traverse(self, mut path: Vec<usize>) -> anyhow::Result<Self> {
        let current = path.pop();

        use SugarExpr::*;
        match current {
            Some(cur) => match self.0 {
                Unit | Var(_) | Type | Lit(_) => bail!("invalid path"),
                Group(e) => match cur {
                    0 => e.traverse(path),
                    _ => bail!("invalid path"),
                },

                Tuple(items) | Sigma(items) | App(items) | Ann(items) | Binding(items)
                | Lambda(items) | Pi(items) | Pipe(items) | Dot(items) => {
                    if let Some(param) = items.get(cur) {
                        param.clone().traverse(path)
                    } else {
                        bail!("invalid path")
                    }
                }
                // SugarExpr::LetIn(_, ty, val, body) => match cur {
                //     0 => ty.traverse(path),
                //     1 => val.traverse(path),
                //     2 => body.traverse(path),
                //     _ => bail!("invalid path"),
                // },
                Module(items1, items2, ..) => match cur {
                    0 => {
                        let cur = path
                            .pop()
                            .ok_or(anyhow!("pointed to mod deps without giving dep"))?;
                        if let Some(param) = items1.get(cur) {
                            param.clone().traverse(path)
                        } else {
                            bail!(
                                "pointed to mod dep {} but mod has only {} deps",
                                cur,
                                items1.len()
                            )
                        }
                    }
                    1 => {
                        let cur = path
                            .pop()
                            .ok_or(anyhow!("pointed to mod items without giving item"))?;
                        if let Some(param) = items2.get(cur) {
                            param.clone().traverse(path)
                        } else {
                            bail!(
                                "pointed to mod item {} but mod has only {} items",
                                cur,
                                items2.len()
                            )
                        }
                    }
                    _ => bail!("invalid path in mod"),
                },
            },

            None => Ok(self),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ResultSugarExpr(pub Result<SugarExpr<ResultSugarExpr>, SugarExprError<ResultSugarExpr>>);

pub fn get_normal_expr(
    expr: ResultSugarExpr,
) -> Result<NormalSugarExpr, SugarExprError<ResultSugarExpr>> {
    match expr.0 {
        Ok(sugar) => sugar
            .try_fmap(|child| get_normal_expr(child))
            .map(NormalSugarExpr),
        Err(e) => Err(e),
    }
}

// enum SubtreeState {
//     Partial(NormalSugarExpr),
//     Recovered(Vec<NormalSugarExpr>),
// }

// TODO: instead of failing if there is something wrong higher up recover by parsing valid subtrees
// pub fn collect_valid_subtrees(expr: ResultSugarExpr) -> SubtreeState {
//     let collect_para = |prev, para_acc| {
//         let normal = match para_acc {
//             Ok(r) => SubtreeState::Partial(NormalSugarExpr(prev)),
//             Err(e) => e.fold,
//         };
//         todo!()
//     };
//     para(expr, collect_para)
// }

// Span is outside because the root expr also has a span. Box is inside because the root expr
// doesn't need to be boxed.
pub type Spanned<T> = (T, Range<usize>);

#[derive(Clone)]
pub struct SpannedResultSugarExpr(
    pub Spanned<Result<SugarExpr<SpannedResultSugarExpr>, SugarExprError<SpannedResultSugarExpr>>>,
);

impl fmt::Debug for SpannedResultSugarExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (inner, span) = &self.0;
        match inner {
            Ok(expr) => write!(f, "{:#?} @ {:?}", expr, span),
            Err(err) => write!(f, "Error: {:#?} @ {:?}", err, span),
        }
    }
}

impl SpannedResultSugarExpr {
    //some of the worst code I have ever written
    // pub fn search(self, token_index: usize) -> Option<Vec<usize>> {
    //     use SugarExpr::*;
    //     let range = self.0.1;
    //     if range.contains(&token_index) {
    //         match self.0.0.clone() {
    //             Ok(node) => match node {
    //                 SugarExpr::Group(a) => a
    //                     .search(token_index)
    //                     .map(|mut x| {
    //                         x.push(0);
    //                         x
    //                     })
    //                     .or(Some(Vec::new())),
    //                 SugarExpr::Sigma(items) => items
    //                     .into_iter()
    //                     .enumerate()
    //                     .fold(None, |acc, (i, item)| {
    //                         if acc.is_none() {
    //                             item.search(token_index).map(|mut x| {
    //                                 x.push(i);
    //                                 x
    //                             })
    //                         } else {
    //                             acc
    //                         }
    //                     })
    //                     .or(Some(Vec::new())),
    //             },
    //
    //             Err(node) => match node {
    //                 FailedLet(a) => a
    //                     .search(token_index)
    //                     .map(|mut x| {
    //                         x.push(0);
    //                         x
    //                     })
    //                     .or(Some(Vec::new())),
    //                 ExprError::Other(a) => a.search(token_index).map(|mut x| {
    //                     x.push(0);
    //                     x
    //                 }),
    //                 _ => Some(Vec::new()),
    //             },
    //         }
    //     } else {
    //         None
    //     }
    // }
    //
    pub fn traverse(
        self,
        mut path: impl Iterator<Item = usize> + std::fmt::Debug,
    ) -> anyhow::Result<Self> {
        use SugarExpr::*;
        let ok = self
            .clone()
            .0
            .0
            .map_err(|_e| anyhow!("invalid syntax tree"))?;
        let current = path.next();

        let err = anyhow!("invalid path: at: {ok:?}, current: {current:?}, still left: {path:?}");
        let err2 = anyhow!("invalid path: stuck at: {ok:?}, current: {current:?}, no path left");

        match current {
            Some(cur) => match ok {
                Var(_) | Lit(_) | Type | Unit => Err(err),
                SugarExpr::App(items)
                | SugarExpr::Lambda(items)
                | SugarExpr::Pi(items)
                | SugarExpr::Tuple(items)
                | SugarExpr::Ann(items)
                | SugarExpr::Binding(items)
                | SugarExpr::Pipe(items)
                | SugarExpr::Sigma(items)
                | SugarExpr::Dot(items) => {
                    let item = items.get(cur).ok_or(err)?;
                    item.clone().traverse(path)
                }
                SugarExpr::Group(e) => match cur {
                    0 => e.traverse(path),
                    _ => Err(err),
                },
                SugarExpr::Module(items1, items2, ..) => match cur {
                    0 => {
                        let cur = path.next().ok_or(err2)?;
                        let param = items1.get(cur).ok_or(err)?;
                        param.clone().traverse(path)
                    }
                    1 => {
                        let cur = path.next().ok_or(err2)?;
                        let param = items2.get(cur).ok_or(err)?;
                        param.clone().traverse(path)
                    }
                    _ => Err(err),
                },
            },

            None => Ok(self),
        }
    }
}

pub fn remove_span(expr: SpannedResultSugarExpr) -> ResultSugarExpr {
    let inner = fmap(expr.0.0, |inner| remove_span(inner));
    ResultSugarExpr(inner)
}
