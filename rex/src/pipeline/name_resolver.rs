use functor_derive::Functor;

use crate::{
    Compile,
    data::expr::{
        Builtin, Defs, Expr, ExprError, ExprF, GDef, GDefs, GExpr, NamedDef, NamedDefs, NamedExpr,
        SpannedExpr, SpannedGExpr, SpannedNamedExpr, VarKind,
    },
    helper::push_new,
    pipeline::desugar::{iter_with_loc, replace_defs},
};

pub struct NameResolver;

impl Compile for NameResolver {
    type Input = NamedDefs;

    type Output = Defs;

    type Error = ResolveError<(Vec<usize>, Context)>;

    fn run(input: Self::Input) -> Result<Self::Output, Self::Error> {
        let defs: Vec<NamedDef> =
            input
                .0
                .into_iter()
                .fold(Vec::new(), |mut defs, GDef { name, ty, val }| {
                    let just_defs = defs
                        .clone()
                        .into_iter()
                        .map(|GDef { name, ty: _, val }| (name.0, val))
                        .collect();
                    let val = replace_defs(val, &just_defs);
                    let ty = ty.map(|x| replace_defs(x, &just_defs));
                    defs.push(GDef { name, ty, val });
                    defs
                });

        let defs = defs
            .into_iter()
            .map(|GDef { name, ty, val }| {
                let ty = ty.map(|x| to_indices(x));
                let val = to_indices(val);
                GDef { name, ty, val }
            })
            .collect();
        Ok(GDefs(defs))
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
pub fn to_indices(expr: SpannedNamedExpr) -> SpannedExpr {
    fn go(expr: SpannedNamedExpr, env: &mut Context, loc: Vec<usize>) -> SpannedExpr {
        let new_expr = match expr.0.0 {
            ExprF::Err(e, items) => {
                let items = iter_with_loc(items, loc)
                    .map(|(x, loc)| Box::new(go(*x, env, loc)))
                    .collect();
                ExprF::Err(e, items)
            }
            ExprF::Var { idx: x } => match x {
                VarKind::Named(x) => {
                    if let Some(pos) = resolve(x.clone(), env) {
                        ExprF::Var { idx: pos }
                    } else {
                        ExprF::Err(ExprError::ResolveFailed(x, (loc, env.clone())), vec![])
                    }
                }
                VarKind::Idx(i) => ExprF::Var { idx: i },
            },
            ExprF::App { func, arg } => ExprF::App {
                func: Box::new(go(*func, env, push_new(loc.clone(), 0))),
                arg: Box::new(go(*arg, env, push_new(loc.clone(), 1))),
            },
            ExprF::Lambda {
                name,
                param_ty,
                body,
            } => {
                let param_ty = Box::new(go(*param_ty, env, push_new(loc.clone(), 1)));
                env.push(name.0.clone());

                let new_name = match name.0 {
                    VarKind::Named(s) => s,
                    VarKind::Idx(s) => s,
                };
                let res = ExprF::Lambda {
                    name: (new_name, name.1),
                    param_ty,
                    body: Box::new(go(*body, env, push_new(loc.clone(), 2))),
                };
                env.pop();
                res
            }
            ExprF::Pi {
                name,
                param_ty,
                ret_ty,
            } => {
                let param_ty = Box::new(go(*param_ty, env, push_new(loc.clone(), 1)));
                env.push(name.0.clone());

                let new_name = match name.0 {
                    VarKind::Named(s) => s,
                    VarKind::Idx(s) => s,
                };
                let res = ExprF::Pi {
                    name: (new_name, name.1),
                    param_ty,
                    ret_ty: Box::new(go(*ret_ty, env, push_new(loc.clone(), 2))),
                };
                env.pop();
                res
            }
            ExprF::Type => ExprF::Type,
            ExprF::Builtin(s) => ExprF::Builtin(s),
        };
        SpannedGExpr((new_expr, expr.0.1))
    }
    go(expr, &mut Vec::new(), Vec::new())
}
