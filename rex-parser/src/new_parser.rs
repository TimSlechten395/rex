use crate::{
    lexer::{AbsoluteIndent, RelativeIndent, Token},
    parser::{NormalSugarExpr, SpannedResultSugarExpr, SugarExpr},
};
use rex_core::{Desugar, Expr, NamedFixExpr};
use rex_core::{DesugarWithNames, FixExpr};

#[derive(Debug, Clone, PartialEq)]
pub struct Items(Vec<Assign>);

#[derive(Debug, Clone, PartialEq)]
pub struct Group(Assign);

#[derive(Debug, Clone, PartialEq)]
pub struct Assign(Vec<Ann>);

#[derive(Debug, Clone, PartialEq)]
pub struct Ann(Vec<Lambda>);

#[derive(Debug, Clone, PartialEq)]
pub struct Lambda(Vec<FnTy>);

#[derive(Debug, Clone, PartialEq)]
pub struct FnTy(Vec<Pipe>);

#[derive(Debug, Clone, PartialEq)]
pub struct Pipe(Vec<Tuple>);

#[derive(Debug, Clone, PartialEq)]
pub struct Tuple(Vec<App>);

#[derive(Debug, Clone, PartialEq)]
pub struct App(Vec<Atom>);

#[derive(Debug, Clone, PartialEq)]
pub enum Atom {
    Group(Group),
    Var(String),
    Unit,
    Type,
    Lit(Lit),
    TODO(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Lit {
    String(String),
    Number(f64),
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenTree {
    Leaf(Token<AbsoluteIndent>),
    Group(Group),
}

pub fn parse(tokens: Vec<Token<AbsoluteIndent>>) -> Items {
    // TODO: This is not at all how we want to handle new_lines
    let ignore_new_lines = tokens
        .into_iter()
        .filter(|x| {
            if let Token::Newline(AbsoluteIndent(n)) = x {
                !(*n > 0 as usize)
            } else {
                true
            }
        })
        .collect::<Vec<_>>();

    let items = ignore_new_lines.split(|x| {
        if let Token::Newline(AbsoluteIndent(n)) = x {
            *n == 0
        } else {
            false
        }
    });

    Items(
        items
            .map(|x| parse_assign(parse_group_start(x.to_vec())))
            .collect(),
    )
}

pub fn parse_assign(tokens: Vec<TokenTree>) -> Assign {
    let splits = tokens.split(|x| *x == TokenTree::Leaf(Token::Assign));
    let expr = splits.map(|x| parse_ann(x.to_vec())).collect();
    Assign(expr)
}

pub fn parse_ann(tokens: Vec<TokenTree>) -> Ann {
    let splits = tokens.split(|x| *x == TokenTree::Leaf(Token::Colon));
    let expr = splits.map(|x| parse_fn(x.to_vec())).collect();
    Ann(expr)
}

pub fn parse_fn(tokens: Vec<TokenTree>) -> Lambda {
    let splits = tokens.split(|x| *x == TokenTree::Leaf(Token::DoubleArrow));
    let expr = splits.map(|x| parse_fn_ty(x.to_vec())).collect();
    Lambda(expr)
}

pub fn parse_fn_ty(tokens: Vec<TokenTree>) -> FnTy {
    let splits = tokens.split(|x| *x == TokenTree::Leaf(Token::Arrow));
    let expr = splits.map(|x| parse_pipe(x.to_vec())).collect();
    FnTy(expr)
}

pub fn parse_pipe(tokens: Vec<TokenTree>) -> Pipe {
    let splits = tokens.split(|x| *x == TokenTree::Leaf(Token::Pipe));
    let expr = splits.map(|x| parse_tuple(x.to_vec())).collect();
    Pipe(expr)
}

pub fn parse_tuple(tokens: Vec<TokenTree>) -> Tuple {
    let splits = tokens.split(|x| *x == TokenTree::Leaf(Token::Comma));
    let expr = splits.map(|x| parse_app(x.to_vec())).collect();
    Tuple(expr)
}

pub fn parse_app(tokens: Vec<TokenTree>) -> App {
    let expr = tokens
        .into_iter()
        .map(|x| match x {
            TokenTree::Leaf(token) => match token {
                Token::Type => Atom::Type,
                Token::Ident(name) => Atom::Var(name),
                Token::String(name) => Atom::Lit(Lit::String(name)),
                Token::Number(num) => Atom::Lit(Lit::Number(num)),
                token => Atom::TODO(format!("{:?}", {})),
            },
            TokenTree::Group(group) => Atom::Group(group),
        })
        .collect();
    App(expr)
}

// interesting this generates token errors instead of ExprErrors
fn parse_group_start(tokens: Vec<Token<AbsoluteIndent>>) -> Vec<TokenTree> {
    let mut trees = Vec::new();

    let mut iter = tokens.into_iter().peekable();

    while let Some(tok) = iter.next() {
        match tok {
            Token::LParen => {
                let group = parse_group_end(&mut iter);
                trees.push(TokenTree::Group(group));
            }
            Token::RParen => panic!("unexpected closing parenthesis"),
            _ => trees.push(TokenTree::Leaf(tok.clone())),
        }
    }

    trees.reverse();
    trees
}

fn parse_group_end<I>(iter: &mut std::iter::Peekable<I>) -> Group
where
    I: Iterator<Item = Token<AbsoluteIndent>>,
{
    let mut inner = Vec::new();

    while let Some(tok) = iter.next() {
        match tok {
            Token::LParen => {
                let group = parse_group_end(iter);
                inner.push(TokenTree::Group(group));
            }
            Token::RParen => return Group(parse_assign(inner)),
            _ => inner.push(TokenTree::Leaf(tok)),
        }
    }
    panic!("unterminated paren group")
}

impl DesugarWithNames for Atom {
    fn desugar_with_names(&self) -> NamedFixExpr {
        match self {
            Atom::Group(group) => group.desugar_with_names(),
            Atom::Var(s) => Expr::Var { idx: s },
            Atom::Unit => Expr::Type,
            Atom::Type => Expr::Type,
            Atom::Lit(lit) => Expr::Type,
            Atom::TODO(_) => Expr::Type,
        }
    }
}

impl DesugarWithNames for App {
    fn desugar_with_names(&self) -> NamedFixExpr {
        if self.0.len() == 0 {
            FixExpr(Expr::Type)
        } else if self.0.len() == 1 {
            self.0.get(0).unwrap().desugar_with_names()
        } else {
            let mut iter = self.0.clone().into_iter().rev();
            let base = iter.next().unwrap().desugar_with_names();

            iter.fold(base, |acc, item| {
                FixExpr(Expr::App {
                    func: Box::new(item.desugar_with_names()),
                    arg: Box::new(acc),
                })
            })
        }
    }
}

impl DesugarWithNames for Pipe {
    fn desugar_with_names(&self) -> NamedFixExpr {
        if self.0.len() == 0 {
            FixExpr(Expr::Type)
        } else if self.0.len() == 1 {
            self.0.get(0).unwrap().desugar_with_names()
        } else {
            let mut iter = self.0.clone().into_iter();
            let base = iter.next().unwrap().desugar_with_names();

            iter.fold(base, |acc, item| {
                FixExpr(Expr::App {
                    func: Box::new(item.desugar_with_names()),
                    arg: Box::new(acc),
                })
            })
        }
    }
}

// TODO:
impl DesugarWithNames for Tuple {
    fn desugar_with_names(&self) -> NamedFixExpr {
        if self.0.len() == 0 {
            FixExpr(Expr::Type)
        } else if self.0.len() == 1 {
            self.0.get(0).unwrap().desugar_with_names()
        } else {
            let f_var = "f".to_string();
            let mut body = Expr::Var { idx: f_var.clone() };

            for (i, e) in items.into_iter().enumerate() {
                let desugared = desugar(*e, loc.clone())?;
                body = Expr::App {
                    // TODO: check this loc
                    func: Box::new(SpannedExprTree((body, loc.clone()))),
                    arg: Box::new(desugared),
                };
            }

            (
                Expr::Lambda {
                    name: f_var.clone(),
                    param_ty: Box::new(SpannedExprTree((Expr::Type, loc.clone()))),
                    body: Box::new(SpannedExprTree((body, loc.clone()))),
                },
                loc.clone(),
            )
        }
    }
}

impl DesugarWithNames for FnTy {
    fn desugar_with_names(&self) -> NamedFixExpr {
        fold_vec_right(&self.0, |_, expr| expr)
    }
}

impl DesugarWithNames for Lambda {
    fn desugar_with_names(&self) -> NamedFixExpr {
        fold_vec_right(&self.0, |binding, expr| {
            let name = match binding.0 {
                Expr::Var { idx } => None,
                Expr::App { func, arg } => None,
                Expr::Lambda {
                    name,
                    param_ty,
                    body,
                } => todo!(),
                Expr::Pi {
                    name,
                    param_ty,
                    ret_ty,
                } => todo!(),
                Expr::Type => todo!(),
            };

            let ty = match binding.0 {
                Expr::Var { idx } => None,
                Expr::App { func, arg } => todo!(),
                Expr::Lambda {
                    name,
                    param_ty,
                    body,
                } => todo!(),
                Expr::Pi {
                    name,
                    param_ty,
                    ret_ty,
                } => todo!(),
                Expr::Type => todo!(),
            };

            FixExpr(Expr::Lambda {
                name: String::new(),
                param_ty: Box::new(Expr::Type),
                body: expr,
            })
        })
    }
}

impl DesugarWithNames for Ann {
    fn desugar_with_names(&self) -> NamedFixExpr {
        fold_vec_right(&self.0, |_, expr| expr)
    }
}

impl DesugarWithNames for Assign {
    fn desugar_with_names(&self) -> NamedFixExpr {
        fold_vec_right(&self.0, |first, second| second) // or some other rule
    }
}
