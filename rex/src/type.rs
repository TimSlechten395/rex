use std::{env::vars, rc::Rc};

use anyhow::Context;
use functor_derive::Functor;
use thiserror::Error;

use crate::{
    // cache::{ExprId, SeaOfNodes},
    data::expr::{Builtin, Expr, ExprF, GExpr, VarKind},
    eval::{beta_reduce, normal_form, shift, weak_head_normal_form},
    helper::push_new,
    pipeline::desugar::create_accessor,
    tools::printer::print_expr,
};

#[derive(Debug, Error, Functor, Clone)]
pub enum TypeError<T> {
    #[error("found unbound variable: {0:?}")]
    UnboundVariable(T, Vec<Rc<Expr>>),

    #[error("type mismatch: expected: {expected:?}, found: {found:?}")]
    TypeMismatch { expected: T, found: T },

    // both the following variants are technically also type mismatches;

    // cannot be Type: Type
    #[error("{0:?} is not a function")]
    NotAFunction(T),

    // must be Type: Type
    #[error("{0:?} is not a type")]
    NotAType(T),

    #[error("Unknown")]
    Unknown(T),
}

// pub fn err_with_nodes(err: TypeError<ExprId>, sea: &SeaOfNodes) -> Option<TypeError<Expr>> {
//     // let new = err.try_fmap(|e| sea.get_tree(e)?);
//     let new = match err {
//         TypeError::UnboundVariable(e, other) => TypeError::UnboundVariable(sea.get_tree(e)?, other),
//         TypeError::NotAFunction(e) => TypeError::NotAFunction(sea.get_tree(e)?),
//
//         TypeError::NotAType(e) => TypeError::NotAType(sea.get_tree(e)?),
//
//         TypeError::TypeMismatch { expected, found } => TypeError::TypeMismatch {
//             expected: sea.get_tree(expected)?,
//             found: sea.get_tree(found)?,
//         },
//     };
//     Some(new)
// }

// Return the type for a term
// we need vars_tys to resolve tys for the vars.
// This only works for terms with no free variables otherwise an unbound variable is returned
// TODO: instead of returning the first type error it finds we should accumulate errors
// Does this always return Expr::Type or Expr::Pi or might it return application or variables
//
// Alternative: Instead of making Pi the binder and substituting directly while type checking we
// could wrap ret_ty in a lambda and then just return App ret_ty arg
pub fn eq(e1: &Expr, e2: &Expr) -> bool {
    if e1 == e2 {
        return true;
    }

    let e1_norm = weak_head_normal_form(e1);
    let e2_norm = weak_head_normal_form(e2);

    // there should be some way to check eq except recursive param
    match (&e1_norm.0, &e2_norm.0) {
        (
            ExprF::App { func, arg },
            ExprF::App {
                func: func2,
                arg: arg2,
            },
        ) => eq(func, func2) && eq(arg, arg2),
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
        ) => eq(param_ty, param_ty2) && eq(body, body2),
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
        ) => eq(param_ty, param_ty2) && eq(ret_ty, ret_ty2),
        (ExprF::Builtin(Builtin::TypeHole), _) => true,
        (_, ExprF::Builtin(Builtin::TypeHole)) => true,
        (e1, e2) => e1 == e2,
    }
}

// input is not Rc because not worth it
pub fn infer_type(
    expr: &Expr,

    ctx: Vec<Rc<Expr>>,
    ty_errors: &mut Vec<TypeError<(Expr, Vec<usize>)>>,
    loc: Vec<usize>,
) -> Result<Expr, TypeError<(Expr, Vec<usize>)>> {
    let ty: ExprF<_, _, _, _> = match &expr.0 {
        ExprF::Err(..) => {
            return Err(TypeError::Unknown((expr.clone(), loc.clone())));
        }
        ExprF::Builtin(b) => match b {
            Builtin::TypeHole => ExprF::Type,
            Builtin::String(_) => ExprF::Builtin(Builtin::StringTy),
            Builtin::StringTy => ExprF::Type,
            Builtin::Num(_) => ExprF::Builtin(Builtin::NumTy),
            Builtin::NumTy => ExprF::Type,
            Builtin::Bool(_) => ExprF::Builtin(Builtin::BoolTy),
            Builtin::BoolTy => ExprF::Type,
            Builtin::StringCmp => ExprF::Pi {
                name: "".to_string(),
                param_ty: Rc::new(GExpr(ExprF::Builtin(Builtin::StringTy))),
                ret_ty: Rc::new(GExpr(ExprF::Pi {
                    name: "".to_string(),
                    param_ty: Rc::new(GExpr(ExprF::Builtin(Builtin::StringTy))),
                    ret_ty: Rc::new(GExpr(ExprF::Builtin(Builtin::BoolTy))),
                })),
            },
            Builtin::NumCmp => ExprF::Pi {
                name: "".to_string(),
                param_ty: Rc::new(GExpr(ExprF::Builtin(Builtin::NumTy))),
                ret_ty: Rc::new(GExpr(ExprF::Pi {
                    name: "".to_string(),
                    param_ty: Rc::new(GExpr(ExprF::Builtin(Builtin::NumTy))),
                    ret_ty: Rc::new(GExpr(ExprF::Builtin(Builtin::BoolTy))),
                })),
            },
            Builtin::BoolCmp => ExprF::Pi {
                name: "".to_string(),
                param_ty: Rc::new(GExpr(ExprF::Builtin(Builtin::BoolTy))),
                ret_ty: Rc::new(GExpr(ExprF::Pi {
                    name: "".to_string(),
                    param_ty: Rc::new(GExpr(ExprF::Builtin(Builtin::BoolTy))),
                    ret_ty: Rc::new(GExpr(ExprF::Builtin(Builtin::BoolTy))),
                })),
            },
            Builtin::Fix => ExprF::Pi {
                name: "".to_string(),
                param_ty: Rc::new(GExpr(ExprF::Type)),
                ret_ty: Rc::new(GExpr(ExprF::Pi {
                    name: "".to_string(),
                    param_ty: Rc::new(GExpr(ExprF::Pi {
                        name: "".to_string(),
                        param_ty: Rc::new(GExpr(ExprF::Var { idx: 0 })),
                        ret_ty: Rc::new(GExpr(ExprF::Var { idx: 1 })),
                    })),
                    ret_ty: Rc::new(GExpr(ExprF::Var { idx: 1 })),
                })),
            }, // Builtin::Struct { map } => ExprF::Pi {
               //     name: (),
               //     param_ty: Box::new(GExpr(ExprF::Builtin(Builtin::StringTy))),
               //     // the return type is the item accessed by inferring the type
               //     ret_ty: Box::new(GExpr(ExprF::Pi {
               //         name: (),
               //         param_ty: Box::new(GExpr(ExprF::Builtin(Builtin::StringTy))),
               //         ret_ty: Box::new(GExpr(ExprF::App {
               //             func: Box::new(GExpr(ExprF::Builtin(Builtin::Accessor))),
               //             arg: Box::new(GExpr(ExprF::Var { idx: 0 as usize })),
               //         })),
               //     })),
               // },
               // Builtin::Accessor { .. } => ExprF::Type,
        },
        // if the type is a variable search for it in context to determine the type
        ExprF::Var { idx } => {
            // just check if the variable has a type
            if let Some(ty) = ctx.get(*idx) {
                ty.0.clone()
            } else {
                // We might need type variables here?
                // This one probably cannot be recovered from
                return Err(TypeError::UnboundVariable((expr.clone(), loc), ctx.clone()));
            }
        }

        // get the function param type and the arg type see if they are the same and then return
        // the function return type
        ExprF::App { func, arg } => {
            let loc0 = push_new(loc.clone(), 0);
            let loc1 = push_new(loc.clone(), 1);

            let func_ty = infer_type(func, ctx.clone(), ty_errors, loc0.clone())?;

            let func_ty_norm = weak_head_normal_form(&func_ty.clone());

            // If func_ty is a pi we can do application
            match &func_ty_norm.0 {
                ExprF::Pi {
                    param_ty, ret_ty, ..
                } => {
                    let arg_ty = &infer_type(arg, ctx.clone(), ty_errors, loc1.clone())?;

                    if !eq(param_ty, arg_ty) {
                        ty_errors.push(TypeError::TypeMismatch {
                            expected: (weak_head_normal_form(param_ty), loc0.clone()),
                            found: (weak_head_normal_form(arg_ty), loc1.clone()),
                        })
                    }

                    beta_reduce(ret_ty, arg).0

                    // ExprF::App {
                    //     func: Box::new(GExpr(ExprF::Lambda {
                    //         name: (),
                    //         param_ty: Box::new(arg_ty_norm),
                    //         body: ret_ty,
                    //     })),
                    //     arg: arg,
                    // }
                }
                other => {
                    // dbg!(other);
                    return Err(TypeError::NotAFunction((func_ty_norm, loc0)));
                }
            }
        }
        // push the type of the parameter and infer the type of the body
        ExprF::Lambda { param_ty, body, .. } => {
            let tyty = &GExpr(ExprF::Type);

            let param_ty_ty = &infer_type(
                param_ty,
                ctx.clone(),
                ty_errors,
                push_new(loc.clone(), 1).clone(),
            )?;

            if !eq(param_ty_ty, tyty) {
                ty_errors.push(TypeError::NotAType((
                    param_ty_ty.clone(),
                    push_new(loc.clone(), 1),
                )));
            }

            let ctx2 = extend_ctx(ctx, (&**param_ty).clone());

            let ret_ty = infer_type(body, ctx2, ty_errors, push_new(loc.clone(), 2))?;
            // why do we normalize here?
            ExprF::Pi {
                name: "".to_string(),
                param_ty: param_ty.clone(),
                ret_ty: Rc::new(ret_ty),
            }
        }
        // Do we need to check if these are types here? maybe check only later when pi type is
        // actually used?
        ExprF::Pi {
            param_ty, ret_ty, ..
        } => {
            let tyty = &GExpr(ExprF::Type);

            let param_ty_ty = &infer_type(
                param_ty,
                ctx.clone(),
                ty_errors,
                push_new(loc.clone(), 1).clone(),
            )?;

            if !eq(param_ty_ty, tyty) {
                ty_errors.push(TypeError::NotAType((
                    param_ty_ty.clone(),
                    push_new(loc.clone(), 1),
                )));
            }

            let ctx2 = extend_ctx(ctx, (&**param_ty).clone());

            let ret_ty_ty = &infer_type(&ret_ty, ctx2, ty_errors, push_new(loc.clone(), 2))?;

            if !eq(ret_ty_ty, tyty) {
                ty_errors.push(TypeError::NotAType((
                    ret_ty_ty.clone(),
                    push_new(loc.clone(), 2),
                )));
            }

            tyty.0.clone()
        }
        // always return type
        ExprF::Type => ExprF::Type,
    };
    Ok(GExpr(ty))
}

fn extend_ctx(mut ctx: Vec<Rc<Expr>>, ty: Expr) -> Vec<Rc<Expr>> {
    ctx.insert(0, Rc::new(ty));
    // TODO: this defeats the purpose of rc
    let new_ctx: Vec<Rc<Expr>> = ctx.iter().map(|t| Rc::new(shift(t, 1, 0))).collect();
    new_ctx
    // ctx
}
