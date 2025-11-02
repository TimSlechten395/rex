use functor_derive::Functor;
use rex_core::{Expr, ExprF, GExpr};
use thiserror::Error;

use crate::{
    eval::weak_head_normal_form,
    push_new,
    sea_nodes::{ExprId, SeaOfNodes},
};

#[derive(Debug, Error)]
#[error("{error} @ {loc:?}")]
pub struct TypeErrorWithLoc<T> {
    pub error: TypeError<T>,
    pub loc: Vec<usize>,
}

#[derive(Debug, Error, Functor)]
pub enum TypeError<T> {
    #[error("found unbound variable: {0:?}")]
    UnboundVariable(T),

    #[error("{0:?} is not a function")]
    NotAFunction(T),

    #[error("type mismatch: expected: {expected:?}, found: {found:?}")]
    TypeMismatch { expected: T, found: T },
}

pub fn err_with_nodes(err: TypeError<ExprId>, sea: &SeaOfNodes) -> Option<TypeError<Expr>> {
    // let new = err.try_fmap(|e| sea.get_tree(e)?);
    let new = match err {
        TypeError::UnboundVariable(e) => TypeError::UnboundVariable(sea.get_tree(e)?),
        TypeError::NotAFunction(e) => TypeError::NotAFunction(sea.get_tree(e)?),
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
pub fn infer_type(
    expr: Expr,

    vars_tys: &mut Vec<Expr>,
    ty_errors: &mut Vec<TypeErrorWithLoc<Expr>>,
    loc: Vec<usize>,
) -> Result<Expr, TypeErrorWithLoc<Expr>> {
    let ty: ExprF<_, _, _> = match expr.0 {
        ExprF::Var { idx } => {
            // just check if the variable has a type
            if let Some(ty) = vars_tys.iter().rev().nth(idx) {
                ty.0.clone()
            } else {
                // We might need type variables here?
                // This one probably cannot be recovered from
                return Err(TypeErrorWithLoc {
                    error: TypeError::UnboundVariable(expr),
                    loc,
                });
            }
        }

        // get the function param type and the arg type see if they are the same and then return
        // the function return type
        ExprF::App { func, arg } => {
            let mut loc0 = loc.clone();
            loc0.push(0);
            let func_ty = infer_type(*func, vars_tys, ty_errors, loc0)?;

            let func_ty_norm = weak_head_normal_form(func_ty);

            // If func_ty is a pi we can do application
            match func_ty_norm.0 {
                ExprF::Pi {
                    param_ty, ret_ty, ..
                } => {
                    let mut loc1 = loc.clone();
                    loc1.push(1);
                    let arg_ty = infer_type(*arg.clone(), vars_tys, ty_errors, loc1)?;
                    let arg_ty_norm = weak_head_normal_form(arg_ty);

                    let param_ty_norm = weak_head_normal_form(*param_ty);

                    if param_ty_norm != arg_ty_norm {
                        ty_errors.push(TypeErrorWithLoc {
                            error: TypeError::TypeMismatch {
                                expected: param_ty_norm,
                                found: arg_ty_norm,
                            },
                            loc,
                        })
                    }
                    ExprF::App { func: ret_ty, arg }
                }
                _ => {
                    return Err(TypeErrorWithLoc {
                        error: TypeError::NotAFunction(func_ty_norm),
                        loc,
                    });
                }
            }
        }
        ExprF::Lambda { param_ty, body, .. } => {
            let tyty = GExpr(ExprF::Type);
            if infer_type(
                *param_ty.clone(),
                vars_tys,
                ty_errors,
                push_new(loc.clone(), 0),
            )? != tyty
            {
                ty_errors.push(TypeErrorWithLoc {
                    error: TypeError::TypeMismatch {
                        expected: tyty,
                        found: *param_ty.clone(),
                    },
                    loc: loc.clone(),
                })
            }

            vars_tys.push(*param_ty.clone());
            let ret_ty = infer_type(*body, vars_tys, ty_errors, push_new(loc.clone(), 0))?;
            let ret_ty_norm = Box::new(weak_head_normal_form(ret_ty));
            ExprF::Pi {
                name: (),
                param_ty: param_ty,
                ret_ty: ret_ty_norm,
            }
        }
        ExprF::Pi {
            param_ty, ret_ty, ..
        } => {
            let mut loc0 = loc.clone();
            loc0.push(0);

            let mut loc1 = loc.clone();
            loc1.push(1);

            let tyty = ExprF::Type;
            if infer_type(*param_ty.clone(), vars_tys, ty_errors, loc0.clone())?.0 != tyty {
                ty_errors.push(TypeErrorWithLoc {
                    error: TypeError::TypeMismatch {
                        expected: GExpr(tyty.clone()),
                        found: *param_ty.clone(),
                    },
                    loc: loc0,
                })
            }

            if infer_type(*ret_ty, vars_tys, ty_errors, loc1.clone())?.0 != tyty.clone() {
                ty_errors.push(TypeErrorWithLoc {
                    error: TypeError::TypeMismatch {
                        expected: GExpr(tyty.clone()),
                        found: *param_ty.clone(),
                    },
                    loc: loc1,
                })
            }

            tyty
        }
        ExprF::Type => ExprF::Type,
    };
    Ok(GExpr(ty))
}
