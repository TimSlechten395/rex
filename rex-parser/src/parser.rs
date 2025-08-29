use std::fmt::Display;

use chumsky::{input::ValueInput, prelude::*};
use either::Either::{self, Right};

use crate::lexer::Token;

// Later all idents will become syntactic sugar for indices
pub type Var = String;

// This could have a variant Common(Expr<SugarExpr>). But we need lambda, pi and var seperate
// anyway
#[derive(Debug, Clone, PartialEq)]
pub enum SugarExpr<T> {
    Var(Var),
    App(T, T), // Function application
    Type,      // Type of all types
    Lambda(Var, T, T),

    Ann(T, T),
    // Pi: (x : Nat) -> Vector Nat x
    Pi(Var, T, T),

    Sigma(Var, T, T),

    // --- Syntactic Sugar Variants ---

    // This is not sugar its necessary for transforming linear text into trees
    Group(T),

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
pub type Spanned<T> = (T, SimpleSpan);

#[derive(Debug, Clone)]
pub struct SpannedSugarExpr(pub Spanned<SugarExpr<Box<SpannedSugarExpr>>>);

pub fn full_parser<'a>()
-> impl Parser<'a, &'a [Token], Vec<SpannedSugarExpr>, extra::Err<Rich<'a, Token>>> {
    parser().separated_by(just(Token::SemiColon)).collect()
}

pub type Span = SimpleSpan;

// TODO: Handle comments better
pub fn parser<'tokens, 'src: 'tokens>()
-> impl Parser<'tokens, &'tokens [Token], SpannedSugarExpr, extra::Err<Rich<'tokens, Token>>> + Clone
{
    let skip_comments = select! {Token::Comment(_)}.ignored().repeated();
    let just_skip_comments = |p| skip_comments.clone().ignore_then(just(p));
    recursive(|expr: Recursive<dyn Parser<'_, _, SpannedSugarExpr, _>>| {
        // `expr` represents the *entire* expression grammar
        // --- 1. Basic Tokens ---
        let r#type = skip_comments
            .clone()
            .ignore_then(just(Token::Type).to(SugarExpr::Type));

        let ident = skip_comments.clone().ignore_then(select! {
        Token::Ident(name) => name});

        let paren_expr = expr
            .clone()
            .delimited_by(
                just_skip_comments(Token::LParen),
                just_skip_comments(Token::RParen),
            )
            .map(|x| SugarExpr::Group(Box::new(x)));

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
                    todo!()
                    // SugarExpr::MultiLambda(params, Box::new(body))
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

        let atom = ident
            .map(SugarExpr::Var)
            .or(r#let)
            .or(lambda)
            .or(sigma)
            .or(r#type)
            .or(paren_expr)
            .map_with(|expr, e| SpannedSugarExpr((expr, e.span())));

        // --- 6. operator precedence forms (Highest binding powers first)
        // Application (Left-Associative Operator)
        let app = atom
            .clone() // The function being applied (e.g., `f`)
            // `foldl` repeatedly parses `projection`s as arguments.
            // This ensures `f x y` parses as `(f x) y`.
            .foldl_with(atom.repeated(), |acc, arg, e| {
                SpannedSugarExpr((SugarExpr::App(Box::new(acc), Box::new(arg)), e.span()))
            });

        let pipe = app.clone().foldl_with(
            just_skip_comments(Token::Pipe).ignore_then(app).repeated(),
            |acc, arg, e| {
                SpannedSugarExpr((SugarExpr::Pipe(Box::new(acc), Box::new(arg)), e.span()))
            },
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
            .foldr_with(
                pipe.clone(),
                // param is the recursive one
                |param, ret_ty, e| match param {
                    Either::Left((name, ty)) => SpannedSugarExpr((
                        SugarExpr::Pi(name, Box::new(ty), Box::new(ret_ty)),
                        e.span(),
                    )),
                    //TODO: Should this be parsed as a different node since it is effectively
                    // just another syntactic sugar
                    Either::Right(ty) => SpannedSugarExpr((
                        SugarExpr::Pi(String::new(), Box::new(ty), Box::new(ret_ty)),
                        e.span(),
                    )),
                },
            );

        // type ann
        let ann = pi.clone().foldl_with(
            just_skip_comments(Token::Colon)
                .ignore_then(pipe)
                .repeated(),
            |val, ty, e| SpannedSugarExpr((SugarExpr::Ann(Box::new(val), Box::new(ty)), e.span())),
        );

        ann.then_ignore(skip_comments)
            .or(skip_comments.map_with(|_, e| SpannedSugarExpr((SugarExpr::Type, e.span()))))
    })
}
