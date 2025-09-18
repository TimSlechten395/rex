use std::fmt::Display;

use anyhow::bail;
use functor_derive::Functor;

// T should be a wrapper around Expr like F<Expr>
// this is incredibly important for a streaming parser
// instead of going text -> tokens -> sugar_ast -> hash cons
// and building the full structure at each step we can also choose to stream instead. This
// massively improve memory usage and cache locality
// Var is stored as Debruijn indices.
// TODO: allow names in Lambda and Pi variants. We could make it generic over B which represent
// binding type but in a lot of variants this will just be ()
#[derive(Debug, Clone, PartialEq, Eq, Hash, Functor)]
pub enum Expr<T, A, B> {
    Var { idx: A },
    App { func: T, arg: T },
    Lambda { name: B, param_ty: T, body: T },
    Pi { name: B, param_ty: T, ret_ty: T },
    Type,
}

impl<T, A, B> Expr<T, A, B> {
    pub fn fold<U>(expr: Expr<T, A, B>, init: U, f: impl Fn(U, T) -> U + Clone) -> U {
        match expr {
            Expr::Var { .. } => init,
            Expr::App { func, arg } => f(f(init, func), arg),
            Expr::Lambda { param_ty, body, .. } => f(f(init, param_ty), body),
            Expr::Pi {
                param_ty, ret_ty, ..
            } => f(f(init, param_ty), ret_ty),
            Expr::Type => init,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExprTree<A, B>(pub Expr<Box<ExprTree<A, B>>, A, B>);

impl<A, B> ExprTree<A, B> {
    pub fn traverse(self, mut path: Vec<usize>) -> anyhow::Result<Self> {
        let current = path.pop();
        match current {
            Some(cur) => match self.0 {
                Expr::Var { .. } => {
                    bail!("invalid path")
                }
                Expr::App { func, arg } => match cur {
                    0 => func.traverse(path),
                    1 => arg.traverse(path),
                    _ => bail!("invalid path"),
                },
                Expr::Lambda { param_ty, body, .. } => match cur {
                    0 => param_ty.traverse(path),
                    1 => body.traverse(path),
                    _ => bail!("invalid path"),
                },
                Expr::Pi {
                    param_ty, ret_ty, ..
                } => match cur {
                    0 => param_ty.traverse(path),
                    1 => ret_ty.traverse(path),
                    _ => bail!("invalid path"),
                },
                Expr::Type => bail!("invalid path"),
            },

            None => Ok(self),
        }
    }

    pub fn cata<R>(self, alg: impl Fn(Expr<R, A, B>) -> R + Clone) -> R {
        let term = self.0;
        let mapped = term.fmap(|subterm| subterm.cata(alg.clone()));
        alg(mapped)
    }
}

#[derive(Debug, Clone)]
pub struct SpannedExprTree<A, B>(pub Spanned<Expr<Box<SpannedExprTree<A, B>>, A, B>>);

impl<A, B> SpannedExprTree<A, B> {
    pub fn traverse(self, mut path: Vec<usize>) -> anyhow::Result<Self> {
        let current = path.pop();
        match current {
            Some(cur) => match self.0.0 {
                Expr::Var { .. } => {
                    bail!("reached var: no more nested expr")
                }
                Expr::App { func, arg } => match cur {
                    0 => func.traverse(path),
                    1 => arg.traverse(path),
                    _ => bail!("reached app: index {cur} is invalid "),
                },
                Expr::Lambda { param_ty, body, .. } => match cur {
                    0 => param_ty.traverse(path),
                    1 => body.traverse(path),
                    _ => bail!("reached lam: index {cur} is invalid "),
                },
                Expr::Pi {
                    param_ty, ret_ty, ..
                } => match cur {
                    0 => param_ty.traverse(path),
                    1 => ret_ty.traverse(path),
                    _ => bail!("reached pi: index {cur} is invalid"),
                },
                Expr::Type => bail!("reached: type: no more nested expr"),
            },

            None => Ok(self),
        }
    }
    pub fn remove_span(self) -> ExprTree<A, B> {
        let expr = self.0.0.fmap(|e| Box::new(e.remove_span()));
        ExprTree(expr)
    }
}

pub type Spanned<T> = (T, Vec<usize>);

impl<A: Display, B: Display> Display for ExprTree<A, B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            Expr::Var { idx: var_id } => write!(f, "{}", var_id),
            // This is not clear how it should display
            Expr::App { func, arg } => {
                if let Expr::App { .. } = arg.0 {
                    write!(f, "{} ({})", func, arg)
                } else {
                    write!(f, "{} {}", func, arg)
                }
            }
            Expr::Lambda {
                name: _,
                param_ty,
                body,
            } => write!(f, "lambda {} => {}", param_ty, body),
            Expr::Pi {
                name: _,
                param_ty,
                ret_ty,
            } => write!(f, "{} -> {}", param_ty, ret_ty),
            Expr::Type => write!(f, "Type"),
        }
    }
}
