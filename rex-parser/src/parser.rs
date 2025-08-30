use std::fmt::Display;

use chumsky::{container::Container, input::ValueInput, prelude::*};
use either::Either::{self, Right};
use functor_derive::Functor;
use smallvec::SmallVec;

use crate::lexer::Token;

#[derive(Debug, Clone, PartialEq, Eq)]
struct MySmallVec<T>(SmallVec<[T; 4]>);
impl<T> Default for MySmallVec<T> {
    fn default() -> Self {
        MySmallVec(SmallVec::new())
    }
}

impl<T> Container<T> for MySmallVec<T> {
    fn push(&mut self, item: T) {
        self.0.push(item);
    }
}

// Later all idents will become syntactic sugar for indices

// This could have a variant Common(Expr<SugarExpr>). But we need lambda, pi and var seperate
// anyway
// TODO: Is this even worth it to have?
#[derive(Debug, Clone, PartialEq)]
pub enum SugarExpr<T> {
    Var(String),
    App(T, T), // Function application
    Type,      // Type of all types

    Ann(T, T),

    Group(T),

    // Multi-argument Lambda (sugar for nested single-arg lambdas)
    // Example: `(lambda (x:T y:U) body)`
    MultiLambda(MySmallVec<(String, Option<T>)>, T),

    // Multi-argument Pi Type (sugar for nested single-arg Pi types)
    // Example: `(Pi (x:T y:U) return_type)`
    MultiPi(MySmallVec<(Option<String>, T)>, T),

    // Let binding sugar: `let name : type = value in body`
    LetIn(String, T, T, T),
    // Pipe operator
    Pipe(T, T),
}

#[derive(Debug, Clone, PartialEq, Functor)]
pub enum ExprError<T> {
    InvalidExpr(Token),
    FailedLet(T),
    Other(T),
}

#[derive(Debug, Clone)]
pub struct NormalSugarExpr(pub SugarExpr<Box<NormalSugarExpr>>);

#[derive(Debug, Clone)]
pub struct ResultSugarExpr(
    pub Result<SugarExpr<Box<ResultSugarExpr>>, ExprError<Box<ResultSugarExpr>>>,
);

// Span is outside because the root expr also has a span. Box is inside because the root expr
// doesn't need to be boxed.
pub type Spanned<T> = (T, SimpleSpan);

#[derive(Debug, Clone)]
pub struct SpannedResultSugarExpr(
    pub  Spanned<
        Result<SugarExpr<Box<SpannedResultSugarExpr>>, ExprError<Box<SpannedResultSugarExpr>>>,
    >,
);

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
        Token::Ident(name) => name};

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

        // fn (x: y) (b: y) -> body)
        let lambda = just(Token::Fn)
            .ignore_then(lambda_arg.repeated().at_least(1).collect::<MySmallVec<_>>())
            .then_ignore(just(Token::Arrow))
            .then(expr.clone()) // Body (can be any expr)
            .map(|(params, body)| SugarExpr::MultiLambda(params, Box::new(body)));

        // let recover_let = any()
        //     .and_is(just(Token::SemiColon).or(just(Token::Let)).not())
        //     .repeated()
        //     .ignore_then(just(Token::SemiColon))
        //     .ignore_then(expr.clone())
        //     .map(|expr| Err(ExprError::FailedLet(Box::new(expr))));
        //

        let r#let = just(Token::Let).ignore_then(
            var_and_type
                .clone()
                .then_ignore(just(Token::Eq))
                .then(expr.clone())
                .then_ignore(just(Token::SemiColon))
                .then(expr.clone())
                .map(|(((var, ty), expr1), expr2)| {
                    Ok(SugarExpr::LetIn(
                        var,
                        Box::new(ty),
                        Box::new(expr1),
                        Box::new(expr2),
                    ))
                }), // .or(recover_let),
        );

        let pi_arg = choice((
            var_and_type
                .clone()
                .delimited_by(just(Token::LParen), just(Token::RParen))
                .map(|(name, ty)| (Some(name), Box::new(ty))),
            expr.clone().map(|ty| (None, Box::new(ty))),
        ));

        let pi = just(Token::Pi)
            .ignore_then(pi_arg.repeated().at_least(1).collect::<MySmallVec<_>>())
            .then_ignore(just(Token::Arrow))
            .then(expr.clone())
            .map(|(params, ret)| SugarExpr::MultiPi(params, Box::new(ret)));

        let atom = choice((
            r#let,
            choice((ident.map(SugarExpr::Var), lambda, r#type, paren_expr)).map(|expr| Ok(expr)),
        ))
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

        pipe

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

impl<T> IntoIterator for SugarExpr<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            SugarExpr::Var(_) | SugarExpr::Type => vec![].into_iter(),

            SugarExpr::App(a, b) | SugarExpr::Pipe(a, b) => vec![a, b].into_iter(),

            SugarExpr::Ann(ty, body) => vec![ty, body].into_iter(),
            SugarExpr::Group(body) => vec![body].into_iter(),

            SugarExpr::MultiLambda(args, body) => {
                todo!()
            }
            SugarExpr::MultiPi(args, body) => {
                todo!()
            }

            SugarExpr::LetIn(_, ty, val, body) => vec![ty, val, body].into_iter(),
        }
    }
}
