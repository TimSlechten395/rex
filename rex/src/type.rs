use std::env::vars;

use anyhow::Context;
use functor_derive::Functor;
use thiserror::Error;

use crate::{
    cache::{ExprId, SeaOfNodes},
    data::expr::{Expr, ExprF, GExpr},
    eval::{beta_reduce, normal_form, shift, weak_head_normal_form},
    helper::push_new,
};

#[derive(Debug, Error, Functor)]
pub enum TypeError<T> {
    #[error("found unbound variable: {0:?}")]
    UnboundVariable(T, Vec<Expr>),

    #[error("type mismatch: expected: {expected:?}, found: {found:?}")]
    TypeMismatch { expected: T, found: T },

    // both the following variants are technically also type mismatches;
    #[error("{0:?} is not a function")]
    NotAFunction(T),

    #[error("{0:?} is not a type")]
    NotAType(T),
}

pub fn err_with_nodes(err: TypeError<ExprId>, sea: &SeaOfNodes) -> Option<TypeError<Expr>> {
    // let new = err.try_fmap(|e| sea.get_tree(e)?);
    let new = match err {
        TypeError::UnboundVariable(e, other) => TypeError::UnboundVariable(sea.get_tree(e)?, other),
        TypeError::NotAFunction(e) => TypeError::NotAFunction(sea.get_tree(e)?),

        TypeError::NotAType(e) => TypeError::NotAType(sea.get_tree(e)?),

        TypeError::TypeMismatch { expected, found } => TypeError::TypeMismatch {
            expected: sea.get_tree(expected)?,
            found: sea.get_tree(found)?,
        },
    };
    Some(new)
}

// Return the type for a term
// we need vars_tys to resolve tys for the vars.
// This only works for terms with no free variables otherwise an unbound variable is returned
// TODO: instead of returning the first type error it finds we should accumulate errors
// Does this always return Expr::Type or Expr::Pi or might it return application or variables
//
// Alternative: Instead of making Pi the binder and substituting directly while type checking we
// could wrap ret_ty in a lambda and then just return App ret_ty arg
pub fn eq(e1: Expr, e2: Expr) -> bool {
    if e1 == e2 {
        return true;
    }
    let e1_norm = weak_head_normal_form(e1);
    let e2_norm = weak_head_normal_form(e2);

    // there should be some way to check eq except recursive param
    match (e1_norm.0, e2_norm.0) {
        (ExprF::Var { idx }, ExprF::Var { idx: idx2 }) => idx == idx2,
        (
            ExprF::App { func, arg },
            ExprF::App {
                func: func2,
                arg: arg2,
            },
        ) => eq(*func, *func2) && eq(*arg, *arg2),
        (
            ExprF::Lambda {
                name: _,
                param_ty,
                body,
            },
            ExprF::Lambda {
                name: _,
                param_ty: param_ty2,
                body: body2,
            },
        ) => eq(*param_ty, *param_ty2) && eq(*body, *body2),
        (
            ExprF::Pi {
                name: _,
                param_ty,
                ret_ty,
            },
            ExprF::Pi {
                name: _,
                param_ty: param_ty2,
                ret_ty: ret_ty2,
            },
        ) => eq(*param_ty, *param_ty2) && eq(*ret_ty, *ret_ty2),
        (ExprF::Type, ExprF::Type) => true,
        _ => false,
    }
}

pub fn infer_type(
    expr: Expr,

    ctx: Vec<Expr>,
    ty_errors: &mut Vec<TypeError<Vec<usize>>>,
    loc: Vec<usize>,
) -> Result<Expr, TypeError<Vec<usize>>> {
    let ty: ExprF<_, _, _> = match expr.0 {
        // if the type is a variable search for it in context to determine the type
        ExprF::Var { idx } => {
            // just check if the variable has a type
            if let Some(ty) = ctx.get(idx) {
                ty.0.clone()
            } else {
                // We might need type variables here?
                // This one probably cannot be recovered from
                return Err(TypeError::UnboundVariable(loc, ctx.clone()));
            }
        }

        // get the function param type and the arg type see if they are the same and then return
        // the function return type
        ExprF::App { func, arg } => {
            let loc0 = push_new(loc.clone(), 0);
            let loc1 = push_new(loc.clone(), 1);

            let func_ty = infer_type(*func.clone(), ctx.clone(), ty_errors, loc0.clone())?;

            let func_ty_norm = weak_head_normal_form(func_ty.clone());

            // If func_ty is a pi we can do application
            match func_ty_norm.clone().0 {
                ExprF::Pi {
                    param_ty, ret_ty, ..
                } => {
                    let arg_ty = infer_type(*arg.clone(), ctx.clone(), ty_errors, loc1.clone())?;

                    if eq(*param_ty, arg_ty) {
                        ty_errors.push(TypeError::TypeMismatch {
                            expected: loc0.clone(),
                            found: loc1.clone(),
                        })
                    }

                    beta_reduce(*ret_ty, *arg).0

                    // ExprF::App {
                    //     func: Box::new(GExpr(ExprF::Lambda {
                    //         name: (),
                    //         param_ty: Box::new(arg_ty_norm),
                    //         body: ret_ty,
                    //     })),
                    //     arg: arg,
                    // }
                }
                _ => {
                    return Err(TypeError::NotAFunction(loc0));
                }
            }
        }
        // push the type of the parameter and infer the type of the body
        ExprF::Lambda { param_ty, body, .. } => {
            let tyty = GExpr(ExprF::Type);

            let param_ty_ty = infer_type(
                *param_ty.clone(),
                ctx.clone(),
                ty_errors,
                push_new(loc.clone(), 0).clone(),
            )?;

            if eq(param_ty_ty, tyty.clone()) {
                ty_errors.push(TypeError::NotAType(push_new(loc.clone(), 0)));
            }

            let ctx2 = extend_ctx(ctx, *param_ty.clone());

            let ret_ty = infer_type(*body, ctx2, ty_errors, push_new(loc.clone(), 1))?;
            // why do we normalize here?
            ExprF::Pi {
                name: (),
                param_ty: param_ty,
                ret_ty: Box::new(ret_ty),
            }
        }
        // Do we need to check if these are types here? maybe check only later when pi type is
        // actually used?
        ExprF::Pi {
            param_ty, ret_ty, ..
        } => {
            let tyty = GExpr(ExprF::Type);

            let param_ty_ty = infer_type(
                *param_ty.clone(),
                ctx.clone(),
                ty_errors,
                push_new(loc.clone(), 0).clone(),
            )?;

            if eq(param_ty_ty.clone(), tyty.clone()) {
                ty_errors.push(TypeError::NotAType(push_new(loc.clone(), 0)));
            }

            let ctx2 = extend_ctx(ctx, *param_ty.clone());

            let ret_ty_ty =
                infer_type(*ret_ty.clone(), ctx2, ty_errors, push_new(loc.clone(), 1))?.0;

            if eq(param_ty_ty, tyty.clone()) {
                ty_errors.push(TypeError::NotAType(push_new(loc.clone(), 1)));
            }

            tyty.0
        }
        // always return type
        ExprF::Type => ExprF::Type,
    };
    Ok(GExpr(ty))
}

fn extend_ctx(mut ctx: Vec<Expr>, ty: Expr) -> Vec<Expr> {
    ctx.insert(0, ty);
    let new_ctx: Vec<Expr> = ctx.iter().map(|t| shift(t.clone(), 1, 0)).collect();
    new_ctx
    // ctx
}
