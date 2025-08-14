use std::fmt::Display;

use chumsky::prelude::*;
use either::Either::{self, Right};

use crate::lexer::Token;

// Later all idents will become syntactic sugar for indices
pub type Var = String;

// This could have a variant Common(Expr<SugarExpr>). But we need lambda, pi and var seperate
// anyway
#[derive(Debug, Clone, PartialEq)]
pub enum SugarExpr<T> {
    Var(Var),
    App(T, T),    // Function application
    Type,         // Type universe U0, U1, etc.
    Atom(String), // Atom literals (e.g., 'hello)
    Lambda(Var, T, T),

    Ann(T, T),
    // Pi: (x : Nat) -> Vector Nat x
    Pi(Var, T, T),

    Sigma(Var, T, T),

    // --- Syntactic Sugar Variants ---

    // Multi-argument Lambda (sugar for nested single-arg lambdas)
    // Example: `(lambda (x:T y:U) body)`
    MultiLambda(Vec<(Var, T)>, T),

    // Multi-argument Pi Type (sugar for nested single-arg Pi types)
    // Example: `(Pi (x:T y:U) return_type)`
    MultiPi(Vec<(Var, T)>, T),

    // Multi-argument Sigma Type (sugar for nested single-arg Sigma types)
    // Example: `(Sigma (x:T y:U) body_type)`
    MultiSigma(Vec<(Var, T)>, T),

    // Infinite Loop Sugar: `loop { body }`
    Loop(T),

    // Let binding sugar: `let name : type = value in body`
    LetIn(Var, T, T, T),
    // Pipe operator
    Pipe(T, T),
}

// Span is outside because the root expr also has a span. Box is inside because the root expr
// doesn't need to be boxed.
type Spanned<T> = (T, SimpleSpan);
pub struct SpannedSugarExpr(pub Spanned<SugarExpr<Box<SpannedSugarExpr>>>);

pub fn full_parser<'a>()
-> impl Parser<'a, &'a [Token], Vec<SpannedSugarExpr>, extra::Err<Rich<'a, Token>>> {
    parser().separated_by(just(Token::SemiColon)).collect()
}

// TODO: Handle comments better
pub fn parser<'a>() -> impl Parser<'a, &'a [Token], SpannedSugarExpr, extra::Err<Rich<'a, Token>>> {
    let skip_comments = select! {Token::Comment(_)}.ignored().repeated();
    let just_skip_comments = |p| skip_comments.clone().ignore_then(just(p));
    recursive(|expr| {
        // `expr` represents the *entire* expression grammar
        // --- 1. Basic Tokens ---
        let basic = skip_comments.clone().ignore_then(select! {
        Token::Atom(name) => SugarExpr::Atom(name),
        Token::Type =>  SugarExpr::Type,
        });

        let ident = skip_comments.clone().ignore_then(select! {
        Token::Ident(name) => name});

        let paren_expr = expr.clone().delimited_by(
            just_skip_comments(Token::LParen),
            just_skip_comments(Token::RParen),
        );

        let var_and_type = ident
            .clone()
            .then_ignore(just_skip_comments(Token::Colon))
            .then(expr.clone());

        // --- 5. Forms (Lambda, Pi, Sigma, Pair) ---
        // Their internal components (e.g., parameter types, body, values) can be any `expr`.
        // These are distinct syntactic constructs, not binary operators.
        // They typically have lower precedence than application.
        //
        let r#loop = just_skip_comments(Token::Loop)
            .ignore_then(expr.clone().delimited_by(
                just_skip_comments(Token::LParen),
                just_skip_comments(Token::RParen),
            ))
            .map(|body| SugarExpr::Loop(Box::new(body)));

        let r#let = just_skip_comments(Token::Let)
            .ignore_then(var_and_type.clone())
            .then_ignore(just_skip_comments(Token::Eq))
            .then(expr.clone())
            .then_ignore(just_skip_comments(Token::In))
            .then(expr.clone())
            .map(|(((var, ty), expr1), expr2)| {
                SugarExpr::LetIn(var, Box::new(ty), Box::new(expr1), Box::new(expr2))
            });

        let lambda = just_skip_comments(Token::Lambda)
            .ignore_then(
                var_and_type
                    .clone()
                    .separated_by(just_skip_comments(Token::Comma))
                    .collect::<Vec<_>>()
                    .delimited_by(
                        just_skip_comments(Token::LParen),
                        just_skip_comments(Token::RParen),
                    ),
            )
            .then_ignore(just_skip_comments(Token::Arrow))
            .then(expr.clone()) // Body (can be any expr)
            .map(|(params, body)| {
                if params.len() == 1 {
                    let (name, ty) = params.into_iter().next().unwrap();
                    SugarExpr::Lambda(name, Box::new(ty), Box::new(body))
                } else if params.len() > 1 {
                    SugarExpr::MultiLambda(params, Box::new(body))
                } else {
                    unreachable!()
                }
            });

        let sigma = just_skip_comments(Token::LParen)
            .ignore_then(var_and_type.clone().delimited_by(
                just_skip_comments(Token::LParen),
                just_skip_comments(Token::RParen),
            )) // Parameter name
            .then_ignore(just_skip_comments(Token::RParen))
            .then_ignore(just_skip_comments(Token::Star))
            .then(expr.clone()) // Second type (can be any expr)
            .map(|((name, ty), second_type)| {
                SugarExpr::Sigma(name, Box::new(ty), Box::new(second_type))
            });

        // no fancy pair syntax for now
        // let pair = just_skip_comments(Token::LParen)
        //     .ignore_then(expr.clone()) // First value (can be any expr)
        //     .then_ignore(just_skip_comments(Token::Comma))
        //     .then(expr.clone()) // Second value (can be any expr)
        //     .then_ignore(just_skip_comments(Token::RParen))
        //     .map(|(val1, val2)| Expr::Pair(Box::new(val1), Box::new(val2)));

        // --- 4. Equality (Lowest Precedence Operator) ---
        // Equality takes any `expr` for its LHS, RHS, and Type.
        // This is the lowest precedence operator, so it appears first in the `choice`.
        // this means the rhs and the type cannot be an equality expression if we want this we need
        // fold

        // --- 6. Atom (Highest Precedence Base Expressions) ---
        // 'Atoms' are expressions that contain no ambiguity
        let atom = choice((
            r#loop,
            r#let,
            lambda,
            sigma,
            basic,
            // pair,
            ident.clone().map(SugarExpr::Var),
            paren_expr,
        ));

        // --- 6. operator precedence forms (Highest binding powers first)
        // Application (Left-Associative Operator)
        let app = atom
            .clone() // The function being applied (e.g., `f`)
            // `foldl` repeatedly parses `projection`s as arguments.
            // This ensures `f x y` parses as `(f x) y`.
            .foldl(atom.repeated(), |acc, arg| {
                SugarExpr::App(Box::new(acc), Box::new(arg))
            });

        let pipe = app.clone().foldl(
            just_skip_comments(Token::Pipe).ignore_then(app).repeated(),
            |acc, arg| SugarExpr::Pipe(Box::new(acc), Box::new(arg)),
        );

        let pi = var_and_type
            .clone()
            .delimited_by(
                just_skip_comments(Token::LParen),
                just_skip_comments(Token::RParen),
            )
            .map(Either::Left)
            .or(pipe.clone().map(Either::Right))
            .then_ignore(just_skip_comments(Token::Arrow))
            .repeated()
            .foldr(
                pipe.clone(),
                // param is the recursive one
                |param, ret_ty| match param {
                    Either::Left((name, ty)) => SugarExpr::Pi(name, Box::new(ty), Box::new(ret_ty)),
                    //TODO: We problably should reserve 0 just for this?
                    Either::Right(ty) => {
                        SugarExpr::Pi(String::new(), Box::new(ty), Box::new(ret_ty))
                    }
                },
            );

        // type ann
        let ann = pi.clone().foldl(
            just_skip_comments(Token::Colon)
                .ignore_then(pipe)
                .repeated(),
            |val, ty| SugarExpr::Ann(Box::new(val), Box::new(ty)),
        );

        let res = ann
            .then_ignore(skip_comments)
            .or(skip_comments.map(|_| SugarExpr::Type));

        res.map_with(|expr, e| (expr, e.span()))
    })
}
