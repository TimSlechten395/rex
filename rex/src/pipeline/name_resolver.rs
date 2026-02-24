use functor_derive::Functor;

use crate::{
    Compile,
    data::expr::{Builtin, Expr, ExprF, GExpr, NamedExpr, VarKind},
    def::Defs,
    helper::push_new,
};

pub struct NameResolver;

impl Compile for NameResolver {
    type Input = NamedExpr;

    type Output = Expr;

    type Error = ResolveError<(Vec<usize>, Context)>;

    fn run(input: Self::Input) -> Result<Self::Output, Self::Error> {
        to_indices(input)
    }
}

// TODO: Do we need to keep VarKind<String, String> because names are only relevant for the other option?
pub type Context = Vec<VarKind<String, String>>;

pub fn resolve(name: String, ctx: &mut Context) -> Option<usize> {
    ctx.iter()
        .rev()
        .position(|n| *n == VarKind::Named(name.clone()))
}

#[derive(thiserror::Error, Functor, Debug)]
pub enum ResolveError<T> {
    #[error("failed to resolve variable {0:?} at {1:?} ")]
    ResolveFailed(String, T),
}

// zipper needed
pub fn to_indices(expr: NamedExpr) -> Result<Expr, ResolveError<(Vec<usize>, Context)>> {
    fn go(
        expr: NamedExpr,
        env: &mut Context,
        loc: Vec<usize>,
    ) -> Result<Expr, ResolveError<(Vec<usize>, Context)>> {
        let expr = match expr.0 {
            ExprF::Var { idx: x } => match x {
                VarKind::Named(x) => {
                    if let Some(pos) = resolve(x.clone(), env) {
                        ExprF::Var { idx: pos }
                    } else {
                        return Err(ResolveError::ResolveFailed(x, (loc, env.clone())));
                    }
                }
                VarKind::Idx(i) => ExprF::Var { idx: i },
            },
            ExprF::App { func, arg } => ExprF::App {
                func: Box::new(go(*func, env, push_new(loc.clone(), 0))?),
                arg: Box::new(go(*arg, env, push_new(loc.clone(), 1))?),
            },
            ExprF::Lambda {
                name,
                param_ty,
                body,
            } => {
                let param_ty = Box::new(go(*param_ty, env, push_new(loc.clone(), 0))?);
                env.push(name.clone());

                let name = match name {
                    VarKind::Named(s) => s,
                    VarKind::Idx(s) => s,
                };
                let res = ExprF::Lambda {
                    name,
                    param_ty,
                    body: Box::new(go(*body, env, push_new(loc.clone(), 1))?),
                };
                env.pop();
                res
            }
            ExprF::Pi {
                name,
                param_ty,
                ret_ty,
            } => {
                let param_ty = Box::new(go(*param_ty, env, push_new(loc.clone(), 0))?);
                env.push(name.clone());

                let name = match name {
                    VarKind::Named(s) => s,
                    VarKind::Idx(s) => s,
                };
                let res = ExprF::Pi {
                    name,
                    param_ty,
                    ret_ty: Box::new(go(*ret_ty, env, push_new(loc.clone(), 1))?),
                };
                env.pop();
                res
            }
            ExprF::Type => ExprF::Type,
            ExprF::Builtin(s) => ExprF::Builtin(s),
        };
        Ok(GExpr(expr))
    }
    go(expr, &mut Vec::new(), Vec::new())
}
