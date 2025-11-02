use core::fmt;
use std::fmt::{Display, Formatter};

use anyhow::bail;
use functor_derive::Functor;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Functor)]
pub enum ExprF<T, A, B> {
    Var { idx: A },
    App { func: T, arg: T },
    Lambda { name: B, param_ty: T, body: T },
    Pi { name: B, param_ty: T, ret_ty: T },
    Type,
}

// TODO: remove parens based on prec and assoc
impl Display for NamedExpr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match &self.0 {
            ExprF::Var { idx } => write!(f, "{idx}"),
            ExprF::Type => write!(f, "Type"),

            ExprF::App { func, arg } => write!(f, "({} {})", func, arg),

            ExprF::Lambda {
                name,
                param_ty,
                body,
            } => match name {
                VarKind::Named(s) => {
                    write!(f, "(({}: {}) => {})", s, param_ty, body)
                }
                VarKind::Idx(_) => {
                    write!(f, "({} => {})", param_ty, body)
                }
            },

            ExprF::Pi {
                name,
                param_ty,
                ret_ty,
            } => match name {
                VarKind::Named(s) => {
                    write!(f, "(({}: {}) -> {})", s, param_ty, ret_ty)
                }
                VarKind::Idx(_) => {
                    write!(f, "({} -> {})", param_ty, ret_ty)
                }
            },
        }
    }
}

impl<T, A, B> ExprF<T, A, B> {
    pub fn fold<U>(expr: ExprF<T, A, B>, init: U, f: impl Fn(U, T) -> U + Clone) -> U {
        match expr {
            ExprF::Var { .. } => init,
            ExprF::App { func, arg } => f(f(init, func), arg),
            ExprF::Lambda { param_ty, body, .. } => f(f(init, param_ty), body),
            ExprF::Pi {
                param_ty, ret_ty, ..
            } => f(f(init, param_ty), ret_ty),
            ExprF::Type => init,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GExpr<A, B>(pub ExprF<Box<GExpr<A, B>>, A, B>);

pub type Expr = GExpr<usize, ()>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VarKind<A, B> {
    Named(A),
    Idx(B),
}

impl Display for VarKind<String, ()> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            VarKind::Named(name) => write!(f, "{name}"),
            VarKind::Idx(_) => write!(f, "$"),
        }
    }
}

impl Display for VarKind<String, usize> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            VarKind::Named(name) => write!(f, "{name}"),
            VarKind::Idx(idx) => write!(f, "${idx}"),
        }
    }
}

pub type NamedExpr = GExpr<VarKind<String, usize>, VarKind<String, ()>>;

pub trait DesugarWithNames {
    fn desugar_with_names(&self) -> NamedExpr;
}

pub trait Desugar {
    fn desugar(&self) -> Expr;
}

impl<A, B> GExpr<A, B> {
    pub fn traverse(self, mut path: Vec<usize>) -> anyhow::Result<Self> {
        let current = path.pop();
        match current {
            Some(cur) => match self.0 {
                ExprF::Var { .. } => {
                    bail!("invalid path")
                }
                ExprF::App { func, arg } => match cur {
                    0 => func.traverse(path),
                    1 => arg.traverse(path),
                    _ => bail!("invalid path"),
                },
                ExprF::Lambda { param_ty, body, .. } => match cur {
                    0 => param_ty.traverse(path),
                    1 => body.traverse(path),
                    _ => bail!("invalid path"),
                },
                ExprF::Pi {
                    param_ty, ret_ty, ..
                } => match cur {
                    0 => param_ty.traverse(path),
                    1 => ret_ty.traverse(path),
                    _ => bail!("invalid path"),
                },
                ExprF::Type => bail!("invalid path"),
            },

            None => Ok(self),
        }
    }

    pub fn cata<R>(self, alg: impl Fn(ExprF<R, A, B>) -> R + Clone) -> R {
        let term = self.0;
        let mapped = term.fmap(|subterm| subterm.cata(alg.clone()));
        alg(mapped)
    }
}

#[derive(Debug, Clone)]
pub struct SpannedExpr<A, B>(pub Spanned<ExprF<Box<SpannedExpr<A, B>>, A, B>, Vec<usize>>);

pub fn remove_span_expr(expr: SpannedNamedExpr) -> NamedExpr {
    let inner = expr.0.0.fmap(|inner| Box::new(remove_span_expr(*inner)));
    GExpr(inner)
}

#[derive(Debug, Clone, Functor, Error)]
pub enum ExprError<T, S> {
    #[error("missing binder: {0:?}")]
    MissingBinder(T, S),

    #[error("missing value: {0:?}")]
    MissingValue(T, S),

    #[error("missing type: {0:?}")]
    MissingType(T, S),

    #[error("invalid binder: {0:?}")]
    InvalidBinder(T, S),

    #[error("invalid binder name: {0:?}")]
    InvalidBinderName(T, S),

    #[error("invalid binder param: {0:?}")]
    InvalidBinderParam(T, S),
}

pub type SpannedNamedExpr = SpannedExpr<VarKind<String, usize>, VarKind<String, ()>>;

pub trait Traverse {
    type Span;
    fn traverse(self, span: Self::Span) -> anyhow::Result<Box<Self>>;
}

pub type Spanned<T, S> = (T, S);

pub trait CompileError<S> {
    fn span(&self) -> S;
}

// here we get the wrong error
pub trait Compile {
    type Output;
    type Error;
    type Span;

    fn run(self) -> Result<Spanned<Self::Output, Self::Span>, Self::Error>;
}

// impl<A, B> Compile for SpannedExpr<A, B>

impl<A, B> Traverse for SpannedExpr<A, B> {
    type Span = Box<dyn Iterator<Item = usize>>;
    fn traverse(self, mut span: Self::Span) -> anyhow::Result<Box<Self>> {
        let current = span.next();
        match current {
            Some(cur) => match self.0.0 {
                ExprF::Var { .. } => {
                    bail!("reached var: no more nested expr")
                }
                ExprF::App { func, arg } => match cur {
                    0 => func.traverse(span),
                    1 => arg.traverse(span),
                    _ => bail!("reached app: index {cur} is invalid "),
                },
                ExprF::Lambda { param_ty, body, .. } => match cur {
                    0 => param_ty.traverse(span),
                    1 => body.traverse(span),
                    _ => bail!("reached lam: index {cur} is invalid "),
                },
                ExprF::Pi {
                    param_ty, ret_ty, ..
                } => match cur {
                    0 => param_ty.traverse(span),
                    1 => ret_ty.traverse(span),
                    _ => bail!("reached pi: index {cur} is invalid"),
                },
                ExprF::Type => bail!("reached type: no more nested expr"),
            },

            None => Ok(Box::new(self)),
        }
    }
}

impl<A, B> SpannedExpr<A, B> {
    // pub fn search(self, token_index: usize) -> Option<Vec<usize>> {
    //         let range = self.0.1.into_range();
    //         if range.contains(&token_index) {
    //             match self.0.0.clone() {
    //                 Ok(node) => match node {
    //                     SugarExpr::App(a, b) => a
    //                         .search(token_index)
    //                         .map(|mut x| {
    //                             x.push(0);
    //                             x
    //                         })
    //                         .or_else(|| {
    //                             b.search(token_index).map(|mut x| {
    //                                 x.push(1);
    //                                 x
    //                             })
    //                         })
    //                         .or(Some(Vec::new())),
    //                     SugarExpr::Ann(a, b) => a
    //                         .search(token_index)
    //                         .map(|mut x| {
    // }

    pub fn remove_span(self) -> GExpr<A, B> {
        let expr = self.0.0.fmap(|e| Box::new(e.remove_span()));
        GExpr(expr)
    }
}
