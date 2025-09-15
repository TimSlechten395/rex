use std::fmt::Display;

use anyhow::anyhow;
use anyhow::bail;
use chumsky::{container::Container, input::ValueInput, prelude::*};
use either::Either::{self, Right};
use functor_derive::Functor;
use std::fmt;

use crate::lexer::Token;

// Later all idents will become syntactic sugar for indices

// This could have a variant Common(Expr<SugarExpr>). But we need lambda, pi and var seperate
// anyway
// TODO: Is this even worth it to have?
#[derive(Debug, Clone, PartialEq, Functor)]
pub enum SugarExpr<T> {
    Var(String),
    App(T, T), // Function application
    Type,      // Type of all types

    Ann(T, T),

    Group(T),

    // Multi-argument Lambda (sugar for nested single-arg lambdas)
    // Example: `(lambda (x:T) (y:U) body)`
    MultiLambda(Vec<(String, Option<T>)>, T),

    // Multi-argument Pi Type (sugar for nested single-arg Pi types)
    // Example: `(Pi (x:T y:U) return_type)`
    MultiPi(Vec<(Option<String>, T)>, T),

    // Let binding sugar: `let name : type = value in body`
    LetIn(String, T, T, T),
    // Pipe operator
    Pipe(T, T),
}

impl<T: Clone> SugarExpr<T> {
    fn fold<U>(&self, init: U, f: impl Fn(U, T) -> U + Clone) -> U {
        match &self {
            SugarExpr::Var(_) => init,
            SugarExpr::App(a, b) => f(f(init, a.clone()), b.clone()),
            SugarExpr::Type => init,
            SugarExpr::Ann(a, b) => f(f(init, a.clone()), b.clone()),
            SugarExpr::Group(a) => f(init, a.clone()),
            SugarExpr::MultiLambda(my_small_vec, b) => f(
                my_small_vec
                    .clone()
                    .into_iter()
                    .filter_map(|x| x.1)
                    .fold(init, f.clone()),
                b.clone(),
            ),
            SugarExpr::MultiPi(my_small_vec, b) => f(
                my_small_vec
                    .clone()
                    .into_iter()
                    .map(|x| x.1)
                    .fold(init, f.clone()),
                b.clone(),
            ),
            SugarExpr::LetIn(_, a, b, c) => f(f(f(init, a.clone()), b.clone()), c.clone()),
            SugarExpr::Pipe(a, b) => f(f(init, a.clone()), b.clone()),
        }
    }
}

pub fn find_node(expr: SpannedResultSugarExpr, idx: usize) -> Option<SpannedResultSugarExpr> {
    if let Some(node) = fold(expr.0.0, None, |acc, t| {
        acc.or_else(|| {
            if t.0.1.into_range().contains(&idx) {
                Some(*t)
            } else {
                None
            }
        })
    }) {
        Some(find_node(node.clone(), idx).unwrap_or(node))
    } else {
        None
    }
}

#[derive(Debug, Clone, PartialEq, Functor)]
pub enum ExprError<T> {
    InvalidExpr(Token),
    FailedLet(T),
    Other(T),
}

impl<T: Clone> ExprError<T> {
    pub fn fold<U>(&self, init: U, f: impl Fn(U, T) -> U + Clone) -> U {
        match &self {
            ExprError::InvalidExpr(token) => init,
            ExprError::FailedLet(a) => f(init, a.clone()),
            ExprError::Other(a) => f(init, a.clone()),
        }
    }
}

pub fn fold<T: Clone, U>(
    expr: Result<SugarExpr<T>, ExprError<T>>,
    init: U,
    f: impl Fn(U, T) -> U + Clone,
) -> U {
    match expr {
        Ok(r) => r.fold(init, f),
        Err(e) => e.fold(init, f),
    }
}

pub fn fmap<T, R>(
    expr: Result<SugarExpr<T>, ExprError<T>>,
    f: impl Fn(T) -> R,
) -> Result<SugarExpr<R>, ExprError<R>> {
    match expr {
        Ok(r) => Ok(r.fmap(f)),
        Err(e) => Err(e.fmap(f)),
    }
}

pub fn cata<B>(
    term: ResultSugarExpr,
    alg: impl Fn(Result<SugarExpr<B>, ExprError<B>>) -> B + Clone,
) -> B {
    let term = term.0;
    let mapped = fmap(term, |subterm| cata(*subterm, alg.clone()));
    alg(mapped)
}

pub fn para<B>(
    term: ResultSugarExpr,
    alg: impl Fn(ResultSugarExpr, Result<SugarExpr<B>, ExprError<B>>) -> B + Clone,
) -> B {
    let term_unfix = term.clone().0;
    let mapped = fmap(term_unfix, |subterm| para(*subterm, alg.clone()));
    alg(term, mapped)
}

#[derive(Debug, Clone)]
pub struct NormalSugarExpr(pub SugarExpr<Box<NormalSugarExpr>>);

impl NormalSugarExpr {
    pub fn traverse(self, mut path: Vec<usize>) -> anyhow::Result<Self> {
        let current = path.pop();
        match current {
            Some(cur) => match self.0 {
                SugarExpr::Var(_) => bail!("invalid path"),
                SugarExpr::App(f, a) => match cur {
                    0 => f.traverse(path),
                    1 => a.traverse(path),
                    _ => bail!("invalid path"),
                },
                SugarExpr::Type => bail!("invalid path"),
                SugarExpr::Ann(e, a) => match cur {
                    0 => e.traverse(path),
                    1 => a.traverse(path),
                    _ => bail!("invalid path"),
                },
                SugarExpr::Group(e) => match cur {
                    0 => e.traverse(path),
                    _ => bail!("invalid path"),
                },
                SugarExpr::MultiLambda(items, body) => match cur {
                    0 => {
                        let cur = path.pop().ok_or(anyhow!(
                            "pointed to multilambda params without giving param"
                        ))?;
                        if let Some(param) = items.get(cur) {
                            if let Some(ty) = param.1.clone() {
                                ty.traverse(path)
                            } else {
                                bail!("no type was given for this lambda param")
                            }
                        } else {
                            bail!(
                                "pointed to {} arg but lambda has only {} args",
                                cur,
                                items.len()
                            )
                        }
                    }
                    1 => body.traverse(path),
                    _ => bail!("invalid path"),
                },

                SugarExpr::MultiPi(items, ret_ty) => match cur {
                    0 => {
                        let cur = path
                            .pop()
                            .ok_or(anyhow!("pointed to multipi params without giving param"))?;
                        if let Some(param) = items.get(cur) {
                            param.1.clone().traverse(path)
                        } else {
                            bail!(
                                "pointed to {} arg but pi has only {} args",
                                cur,
                                items.len()
                            )
                        }
                    }
                    1 => ret_ty.traverse(path),
                    _ => bail!("invalid path"),
                },
                SugarExpr::LetIn(_, ty, val, body) => match cur {
                    0 => ty.traverse(path),
                    1 => val.traverse(path),
                    2 => body.traverse(path),
                    _ => bail!("invalid path"),
                },
                SugarExpr::Pipe(f, a) => match cur {
                    0 => f.traverse(path),
                    1 => a.traverse(path),
                    _ => bail!("invalid path"),
                },
            },

            None => Ok(self),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ResultSugarExpr(
    pub Result<SugarExpr<Box<ResultSugarExpr>>, ExprError<Box<ResultSugarExpr>>>,
);

pub fn is_valid(expr: ResultSugarExpr) -> bool {
    let is_valid_cata = |sub| fold(sub, true, |acc, next| acc && next);
    cata(expr, is_valid_cata)
}

pub fn get_normal_expr(expr: ResultSugarExpr) -> Option<NormalSugarExpr> {
    match expr.0 {
        Ok(sugar) => sugar
            .try_fmap(|child| get_normal_expr(*child).ok_or(()).map(Box::new))
            .ok()
            .map(NormalSugarExpr),
        Err(_) => None,
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
pub type Spanned<T> = (T, SimpleSpan);

#[derive(Clone)]
pub struct SpannedResultSugarExpr(
    pub  Spanned<
        Result<SugarExpr<Box<SpannedResultSugarExpr>>, ExprError<Box<SpannedResultSugarExpr>>>,
    >,
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
    pub fn traverse(self, mut path: Vec<usize>) -> anyhow::Result<Self> {
        let ok = self
            .clone()
            .0
            .0
            .map_err(|_e| anyhow!("invalid syntax tree"))?;
        let current = path.pop();
        match current {
            Some(cur) => match ok {
                SugarExpr::Var(_) => bail!("invalid path"),
                SugarExpr::App(f, a) => match cur {
                    0 => f.traverse(path),
                    1 => a.traverse(path),
                    _ => bail!("invalid path"),
                },
                SugarExpr::Type => bail!("invalid path"),
                SugarExpr::Ann(e, a) => match cur {
                    0 => e.traverse(path),
                    1 => a.traverse(path),
                    _ => bail!("invalid path"),
                },
                SugarExpr::Group(e) => match cur {
                    0 => e.traverse(path),
                    _ => bail!("invalid path"),
                },
                SugarExpr::MultiLambda(items, body) => match cur {
                    0 => {
                        let cur = path.pop().ok_or(anyhow!(
                            "pointed to multilambda params without giving param"
                        ))?;
                        if let Some(param) = items.get(cur) {
                            if let Some(ty) = param.1.clone() {
                                ty.traverse(path)
                            } else {
                                bail!("no type was given for this lambda param")
                            }
                        } else {
                            bail!(
                                "pointed to {} arg but lambda has only {} args",
                                cur,
                                items.len()
                            )
                        }
                    }
                    1 => body.traverse(path),
                    _ => bail!("invalid path"),
                },

                SugarExpr::MultiPi(items, ret_ty) => match cur {
                    0 => {
                        let cur = path
                            .pop()
                            .ok_or(anyhow!("pointed to multipi params without giving param"))?;
                        if let Some(param) = items.get(cur) {
                            param.1.clone().traverse(path)
                        } else {
                            bail!(
                                "pointed to {} arg but pi has only {} args",
                                cur,
                                items.len()
                            )
                        }
                    }
                    1 => ret_ty.traverse(path),
                    _ => bail!("invalid path"),
                },
                SugarExpr::LetIn(_, ty, val, body) => match cur {
                    0 => ty.traverse(path),
                    1 => val.traverse(path),
                    2 => body.traverse(path),
                    _ => bail!("invalid path"),
                },
                SugarExpr::Pipe(f, a) => match cur {
                    0 => f.traverse(path),
                    1 => a.traverse(path),
                    _ => bail!("invalid path"),
                },
            },

            None => Ok(self),
        }
    }
}

pub fn remove_span(expr: SpannedResultSugarExpr) -> ResultSugarExpr {
    let inner = fmap(expr.0.0, |inner| Box::new(remove_span(*inner)));
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

pub type Span = SimpleSpan;

// TODO: Use recursion schemes
pub fn parser<'tokens, 'src: 'tokens>()
-> impl Parser<'tokens, &'tokens [Token], SpannedResultSugarExpr, extra::Err<Rich<'tokens, Token>>>
+ Clone {
    recursive(|expr| {
        // `expr` represents the *entire* expression grammar
        // --- 1. Basic Tokens ---
        let r#type = just(Token::Type).to(SugarExpr::Type);
        // same as base

        let ident = select! {
        Token::Ident(name) => name
        };

        let paren_expr = expr
            .clone()
            .delimited_by(just(Token::LParen), just(Token::RParen))
            .map(|x| SugarExpr::Group(Box::new(x)));

        let var_and_type = ident
            .clone()
            .then_ignore(just(Token::Colon))
            .then(expr.clone());

        // type annotation is optional
        let lambda_arg = choice((
            var_and_type
                .clone()
                .delimited_by(just(Token::LParen), just(Token::RParen))
                .map(|(name, ty)| (name, Some(Box::new(ty)))),
            ident.clone().map(|name| (name, None)),
        ));

        // fn (x: y) (b: y) => body)
        let lambda = just(Token::Fn)
            .ignore_then(lambda_arg.repeated().at_least(1).collect::<Vec<_>>())
            .then_ignore(just(Token::DoubleArrow))
            .then(expr.clone()) // Body (can be any expr)
            .map(|(params, body)| SugarExpr::MultiLambda(params, Box::new(body)));

        // let recover_let = any()
        //     .and_is(just(Token::SemiColon).or(just(Token::Let)).not())
        //     .repeated()
        //     .ignore_then(just(Token::SemiColon))
        //     .ignore_then(expr.clone())
        //     .map(|expr| Err(ExprError::FailedLet(Box::new(expr))));
        //

        let atom = choice((ident.map(SugarExpr::Var), lambda, r#type, paren_expr))
            .map(|expr| Ok(expr))
            .map_with(|expr, e| SpannedResultSugarExpr((expr, e.span())));

        // --- operator precedence forms (Highest binding powers first)
        let app = atom.clone().foldl_with(atom.repeated(), |acc, arg, e| {
            SpannedResultSugarExpr((Ok(SugarExpr::App(Box::new(acc), Box::new(arg))), e.span()))
        });

        let pipe = app.clone().foldl_with(
            just(Token::Pipe).ignore_then(app).repeated(),
            |acc, arg, e| {
                SpannedResultSugarExpr((
                    Ok(SugarExpr::Pipe(Box::new(acc), Box::new(arg))),
                    e.span(),
                ))
            },
        );

        let pi_arg = choice((
            var_and_type
                .clone()
                .delimited_by(just(Token::LParen), just(Token::RParen))
                .map(|(name, ty)| (Some(name), Box::new(ty))),
            pipe.clone().map(|ty| (None, Box::new(ty))),
        ));

        let pi = pi_arg
            .then_ignore(just(Token::Arrow))
            .repeated()
            .collect::<Vec<(Option<String>, Box<SpannedResultSugarExpr>)>>()
            .then(pipe.clone())
            .map_with(|(params, ret), e| {
                if params.len() > 0 {
                    SpannedResultSugarExpr((
                        Ok(SugarExpr::MultiPi(params, Box::new(ret))),
                        e.span(),
                    ))
                } else {
                    ret
                }
            });

        // TODO: If we make let an atom the parser seems to blow up. Figure out why this is the
        // case
        let r#let = just(Token::Let).ignore_then(
            var_and_type
                .clone()
                .then_ignore(just(Token::Eq))
                .then(expr.clone())
                .then_ignore(just(Token::SemiColon))
                .then(expr.clone())
                .map_with(|(((var, ty), expr1), expr2), e| {
                    SpannedResultSugarExpr((
                        Ok(SugarExpr::LetIn(
                            var,
                            Box::new(ty),
                            Box::new(expr1),
                            Box::new(expr2),
                        )),
                        e.span(),
                    ))
                }), // .or(recover_let),
        );
        choice((pi, r#let))

        // let ann = pi.clone().foldl_with(
        //     just(Token::Colon).ignore_then(pi).repeated(),
        //     |val, ty, e| {
        //         SpannedResultSugarExpr((Ok(SugarExpr::Ann(Box::new(val), Box::new(ty))), e.span()))
        //     },
        // );
        //
        // ann
    })
}
