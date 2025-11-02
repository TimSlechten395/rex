use std::fmt::Display;
use std::fmt::Formatter;
use std::ops::Range;

use anyhow::anyhow;
use anyhow::bail;
use functor_derive::Functor;
use num_bigint::BigUint;
use std::fmt;
use thiserror::Error;

use crate::lexer::AbsoluteIndent;
use crate::lexer::Token;

// Later all idents will become syntactic sugar for indices

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

// returns result because whole tree might be invalid
// pub fn clean(ast: SpannedResultSugarExpr) -> ResultSugarExpr {
//     let cleaned = match ast.0.0 {
//         Ok(ast) => Ok(ast.fmap(|ast| Box::new(clean(*ast)))),
//         Err(err_ast) => Err(err_ast.fmap(|ast| Box::new(clean(*ast)))),
//     };
//
//     ResultSugarExpr(cleaned)
// }

// TODO: Use recursion schemes
// pub fn parser<'tokens, 'src: 'tokens>() -> impl Parser<
//     'tokens,
//     &'tokens [Token<AbsoluteIndent>],
//     SpannedResultSugarExpr,
//     extra::Err<Rich<'tokens, Token<AbsoluteIndent>>>,
// > + Clone {
//     recursive(|expr| {
//         // `expr` represents the *entire* expression grammar
//         // --- 1. Basic Tokens ---
//         let r#type = just(Token::Type).to(SugarExpr::Type);
//         // same as base
//
//         let ident = select! {
//         Token::Ident(name) => name
//         };
//
//         let paren_expr = expr
//             .clone()
//             .delimited_by(just(Token::LParen), just(Token::RParen))
//             .map(|x| SugarExpr::Group(Box::new(x)));
//
//         let var_and_type = ident
//             .clone()
//             .then_ignore(just(Token::Colon))
//             .then(expr.clone());
//
//         // type annotation is optional
//         let lambda_arg = choice((
//             var_and_type
//                 .clone()
//                 .delimited_by(just(Token::LParen), just(Token::RParen))
//                 .map(|(name, ty)| (name, Some(Box::new(ty)))),
//             ident.clone().map(|name| (name, None)),
//         ));
//
//         // fn (x: y) (b: y) => body)
//         let lambda = just(Token::Fn)
//             .ignore_then(lambda_arg.repeated().at_least(1).collect::<Vec<_>>())
//             .then_ignore(just(Token::DoubleArrow))
//             .then(expr.clone()) // Body (can be any expr)
//             .map(|(params, body)| SugarExpr::MultiLambda(params, Box::new(body)));
//
//         let atom = choice((ident.map(SugarExpr::Var), lambda, r#type, paren_expr))
//             .map(|expr| Ok(expr))
//             .map_with(|expr, e| SpannedResultSugarExpr((expr, e.span())));
//
//         // --- operator precedence forms (Highest binding powers first)
//         let app = atom.clone().foldl_with(atom.repeated(), |acc, arg, e| {
//             SpannedResultSugarExpr((Ok(SugarExpr::App(Box::new(acc), Box::new(arg))), e.span()))
//         });
//
//         let pipe = app.clone().foldl_with(
//             just(Token::Pipe).ignore_then(app).repeated(),
//             |acc, arg, e| {
//                 SpannedResultSugarExpr((
//                     Ok(SugarExpr::Pipe(Box::new(acc), Box::new(arg))),
//                     e.span(),
//                 ))
//             },
//         );
//         let pi_arg = choice((
//             var_and_type
//                 .clone()
//                 .delimited_by(just(Token::LParen), just(Token::RParen))
//                 .map(|(name, ty)| (Some(name), Box::new(ty))),
//             pipe.clone().map(|ty| (None, Box::new(ty))),
//         ));
//
//         let pi = pi_arg
//             .then_ignore(just(Token::Arrow))
//             .repeated()
//             .collect::<Vec<(Option<String>, Box<SpannedResultSugarExpr>)>>()
//             .then(pipe.clone())
//             .map_with(|(params, ret), e| {
//                 if params.len() > 0 {
//                     SpannedResultSugarExpr((
//                         Ok(SugarExpr::MultiPi(params, Box::new(ret))),
//                         e.span(),
//                     ))
//                 } else {
//                     ret
//                 }
//             });
//
//         let sigma = pi
//             .clone()
//             .separated_by(just(Token::Comma))
//             .at_least(1)
//             .collect::<Vec<SpannedResultSugarExpr>>()
//             .then(just(Token::Comma).or_not())
//             .map_with(|(items, comma), e| {
//                 let mut items: Vec<_> = items.into_iter().map(|x| Box::new(x)).collect();
//                 if items.len() > 1 || comma.is_some() {
//                     SpannedResultSugarExpr((Ok(SugarExpr::MultiSigma(items)), e.span()))
//                 } else {
//                     *items.pop().unwrap()
//                 }
//             });
//
//         let recover_let = any()
//             .and_is(just(Token::SemiColon).or(just(Token::Let)).not())
//             .repeated()
//             .ignore_then(just(Token::SemiColon))
//             .ignore_then(expr.clone())
//             .map(|expr| Err(ExprError::FailedLet(Box::new(expr))))
//             .map_with(|expr, e| SpannedResultSugarExpr((expr, e.span())));
//
//         // TODO: If we make let an atom the parser seems to blow up. Figure out why this is the
//         // case
//         let r#let = just(Token::Let).ignore_then(
//             var_and_type
//                 .clone()
//                 .then_ignore(just(Token::Eq))
//                 .then(expr.clone())
//                 .then_ignore(just(Token::SemiColon))
//                 .then(expr.clone())
//                 .map_with(|(((var, ty), expr1), expr2), e| {
//                     SpannedResultSugarExpr((
//                         Ok(SugarExpr::LetIn(
//                             var,
//                             Box::new(ty),
//                             Box::new(expr1),
//                             Box::new(expr2),
//                         )),
//                         e.span(),
//                     ))
//                 })
//                 .or(recover_let),
//         );
//         choice((sigma, r#let))
//
//         // let ann = pi.clone().foldl_with(
//         //     just(Token::Colon).ignore_then(pi).repeated(),
//         //     |val, ty, e| {
//         //         SpannedResultSugarExpr((Ok(SugarExpr::Ann(Box::new(val), Box::new(ty))), e.span()))
//         //     },
//         // );
//         //
//         // ann
//     })
// }
//
