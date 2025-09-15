use chumsky::{prelude::*, text::ascii::ident};

// pub fn get_definition<'a>()
// -> impl Parser<'a, &'a [Token], ((String, SugarExpr), SugarExpr), extra::Err<Rich<'a, Token>>> {
//     let ident = select! {
//     Token::Ident(name) => name,};
//
//     just(Token::Let)
//         .ignore_then(ident)
//         .then_ignore(just(Token::Colon))
//         .then(crate::parser())
//         .then_ignore(just(Token::Eq))
//         .then(crate::parser())
// }
