use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Formatter;
use std::ops::Range;
use std::ops::RangeInclusive;

use anyhow::anyhow;
use anyhow::bail;
use functor_derive::Functor;
use num_bigint::BigUint;
use std::fmt;
use thiserror::Error;

use crate::Traverse;
use crate::data::tokens::AbsoluteIndent;
use crate::data::tokens::GToken;
use crate::data::tokens::ValidToken;
use crate::helper::push_new;
use crate::pipeline::desugar::iter_with_loc;

// Later all idents will become syntactic sugar for indices

// TODO: Is this even worth it to have?
#[derive(Debug, Clone, PartialEq, Functor)]
pub enum Ast<T> {
    Var(String),
    App(Vec<T>), // Function application
    Type,        // Type of all types
    Unit,
    Lit(LitKind),
    Dot(Vec<T>),

    Ann(Vec<T>),     // ':'
    Binding(Vec<T>), // '='

    Group(Box<T>),

    // first param is types of module dependencies second is the items in the module
    // to be converted to product type
    // pub/priv is done with an extra module that just takes the full module and returns a smaller
    // module
    // always depends one self
    // Module(Vec<T>, Vec<T>),

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
    Error(AstError, Vec<T>),
}

impl<T: Display> Display for Ast<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        use Ast::*;

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
            Hole => write!(f, "_"),
            // Module(deps, items) => {
            //     write!(f, "module(deps=[")?;
            //     join_fmt(f, deps, "\n")?;
            //     write!(f, "], items=[\n def ")?;
            //     join_fmt(f, items, "\n def ")?;
            //     write!(f, "]")
            // }
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

impl<T: Clone> Ast<T> {
    pub fn fold<U>(&self, init: U, f: impl Fn(U, T) -> U + Clone) -> U {
        match &self {
            Ast::Unit | Ast::Var(_) | Ast::Type | Ast::Lit(_) => init,
            Ast::Group(a) => f(init, *a.clone()),
            Ast::App(items)
            | Ast::Dot(items)
            | Ast::Lambda(items)
            | Ast::Pi(items)
            | Ast::Binding(items)
            | Ast::Ann(items)
            | Ast::Tuple(items)
            | Ast::Sigma(items)
            | Ast::Error(_, items)
            | Ast::Pipe(items) => items.clone().into_iter().fold(init, f.clone()),
            // Ast::Module(items1, items2, ..) => items2
            //     .clone()
            //     .into_iter()
            //     .fold(items1.clone().into_iter().fold(init, f.clone()), f.clone()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Error)]
pub enum AstError {
    #[error("Invalid expression: found tokens {0:?}")]
    InvalidExpr(Vec<ValidToken<AbsoluteIndent>>),

    #[error("Unknown error")]
    Unknown,

    #[error("Invalid token: found tokens {0:?}")]
    InvalidToken(ValidToken<AbsoluteIndent>),
}

#[derive(Debug, Clone)]
pub struct FixAst(pub Ast<FixAst>);

impl Display for FixAst {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        // Delegate to the inner SugarExpr's Display implementation
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone)]
pub struct SpannedFixAst(pub Spanned<Ast<SpannedFixAst>>);

pub fn traverse_list(
    ast: Vec<SpannedFixAst>,
    path: Vec<usize>,
) -> anyhow::Result<Box<SpannedFixAst>> {
    let mut iter = path.into_iter();
    let Some(next) = iter.next() else {
        bail!("path was completely empty")
    };
    let Some(ast) = ast.get(next) else {
        bail!("index out of bounds: len: {} got: {}", ast.len(), next)
    };
    ast.clone().traverse(iter.collect())
}
impl Traverse for SpannedFixAst {
    type Span = Vec<usize>;

    fn traverse(self, path: Vec<usize>) -> anyhow::Result<Box<Self>> {
        let mut path = path.into_iter();
        let current = path.next();

        use Ast::*;
        match current {
            Some(cur) => match self.0.0.clone() {
                Unit | Var(_) | Type | Lit(_) => bail!("invalid path in {:?}", &self.0.0),
                Group(e) => match cur {
                    0 => e.traverse(path.collect()),
                    _ => bail!("invalid path in {:?}, cur: {:?} ", &self.0.0, cur),
                },

                Tuple(items)
                | Sigma(items)
                | App(items)
                | Ann(items)
                | Binding(items)
                | Lambda(items)
                | Pi(items)
                | Pipe(items)
                | Dot(items)
                | Error(_, items) => {
                    if let Some(param) = items.get(cur) {
                        param.clone().traverse(path.collect())
                    } else {
                        bail!(
                            "invalid path in {:?}, cur: {:?}, max: {:?}",
                            &self.0.0,
                            cur,
                            items.len()
                        )
                    }
                } // SugarExpr::LetIn(_, ty, val, body) => match cur {
                  //     0 => ty.traverse(path),
                  //     1 => val.traverse(path),
                  //     2 => body.traverse(path),
                  //     _ => bail!("invalid path"),
                  // },
                  // Module(items1, items2, ..) => match cur { 0 => {
                  //         let cur = match path.next() {
                  //             Some(ok) => ok,
                  //             None => return Ok(Box::new(self)),
                  //         };
                  //         if let Some(param) = items1.get(cur) {
                  //             param.clone().traverse(path.collect())
                  //         } else {
                  //             bail!(
                  //                 "pointed to mod dep {} but mod has only {} deps",
                  //                 cur,
                  //                 items1.len()
                  //             )
                  //         }
                  //     }
                  //     1 => {
                  //         let cur = match path.next() {
                  //             Some(ok) => ok,
                  //             None => return Ok(Box::new(self)),
                  //         };
                  //
                  //         if let Some(param) = items2.get(cur) {
                  //             param.clone().traverse(path.collect())
                  //         } else {
                  //             bail!(
                  //                 "pointed to mod item {} but mod has only {} items",
                  //                 cur,
                  //                 items2.len()
                  //             )
                  //         }
                  //     }
                  //     _ => bail!("invalid path in mod"),
                  // },
            },

            None => Ok(Box::new(self)),
        }
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
pub type Spanned<T> = (T, RangeInclusive<usize>);

pub fn search(ast: SpannedFixAst, loc: usize, cur_loc: Vec<usize>) -> Vec<Vec<usize>> {
    use Ast::*;
    let range = ast.0.1;
    if range.contains(&loc) {
        let mut locs = match ast.0.0.clone() {
            Var(_) | Type | Unit | Lit(_) => vec![],
            App(items)
            | Dot(items)
            | Ann(items)
            | Binding(items)
            | Lambda(items)
            | Pi(items)
            | Tuple(items)
            | Sigma(items)
            | Pipe(items)
            | Error(_, items) => iter_with_loc(items, cur_loc.clone())
                .map(|x| search(x.0, loc, x.1))
                .reduce(|mut acc, e| {
                    acc.extend(e);
                    acc
                })
                .unwrap_or(Vec::new()),
            Group(item) => search(*item, loc, push_new(cur_loc.clone(), 0)),
        };
        if locs.is_empty() {
            locs.push(cur_loc)
        }
        locs
    } else {
        vec![]
    }
}

pub fn search_list(ast: Vec<SpannedFixAst>, loc: usize) -> Vec<Vec<usize>> {
    ast.clone()
        .into_iter()
        .enumerate()
        .map(|(i, ast)| search(ast, loc, vec![i]))
        .find(|x| !x.is_empty())
        .unwrap_or(vec![])
}

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
impl SpannedFixAst {
    pub fn remove_span(self) -> FixAst {
        let inner = self.0.0.fmap(|inner| inner.remove_span());
        FixAst(inner)
    }
}
