use rex_core::{Expr, ExprTree};
use thiserror::Error;

use crate::{
    eval::{normalize, strong_normalize},
    sea_nodes::{NodeId, SeaOfNodes},
};

#[derive(Debug, Error)]
#[error("{error} @ {loc:?}")]
pub struct TypeErrorWithLoc<T> {
    pub error: TypeError<T>,
    pub loc: Vec<usize>,
}

#[derive(Debug, Error)]
pub enum TypeError<T> {
    #[error("found unbound variable: {0:?}")]
    UnboundVariable(T),

    #[error("{0:?} is not a function")]
    NotAFunction(T),

    #[error("type mismatch: expected: {expected:?}, found: {found:?}")]
    TypeMismatch { expected: T, found: T },
}

pub fn err_with_nodes(
    err: TypeError<NodeId>,
    sea: &SeaOfNodes,
) -> Option<TypeError<ExprTree<usize, ()>>> {
    // nice functor
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
// Unfortunalty we need vars_tys to resolve tys for the vars. If vars were globally unique this
// would not be a problem
// TODO: check if is is possible to use a generic Expr<T> instead. I assume it is impossible
// because there is no generic way to traverse the expression.
// HACK: This only works for terms with no free variables otherwise an unboundvariable is returned
pub fn infer_type(
    expr_id: NodeId,
    sea: &mut SeaOfNodes,

    // This is gonna be a fun monad
    vars_tys: &mut Vec<NodeId>,
    ty_errors: &mut Vec<TypeErrorWithLoc<NodeId>>,
    loc: Vec<usize>,
) -> Result<NodeId, TypeErrorWithLoc<NodeId>> {
    let expr = sea.get_node(expr_id).unwrap().clone();
    let ty = match expr {
        Expr::Var { idx } => {
            // just check if the variable has a type
            if let Some(ty) = vars_tys.iter().rev().nth(idx) {
                Ok(*ty)
            } else {
                // We might need type variables here?
                Err(TypeErrorWithLoc {
                    error: TypeError::UnboundVariable(expr_id),
                    loc,
                })
            }
        }

        // get the function param type and the arg type see if they are the same and then return
        // the function return type
        Expr::App { func, arg } => {
            let mut loc0 = loc.clone();
            loc0.push(0);
            let func_ty = infer_type(func, sea, vars_tys, ty_errors, loc0)?;

            let func_ty_norm = normalize(func_ty, sea);

            // If func_ty is a pi we can do application
            match sea.get_node(func_ty_norm).unwrap().clone() {
                Expr::Pi {
                    param_ty, ret_ty, ..
                } => {
                    let mut loc1 = loc.clone();
                    loc1.push(1);
                    let arg_ty = infer_type(arg, sea, vars_tys, ty_errors, loc1)?;
                    let arg_ty_norm = normalize(arg_ty, sea);

                    let param_ty_norm = normalize(param_ty, sea);

                    if param_ty_norm != arg_ty_norm {
                        ty_errors.push(TypeErrorWithLoc {
                            error: TypeError::TypeMismatch {
                                expected: param_ty_norm,
                                found: arg_ty_norm,
                            },
                            loc,
                        })
                    }
                    Ok(sea.add_node(Expr::App { func: ret_ty, arg }))
                }
                _ => Err(TypeErrorWithLoc {
                    error: TypeError::NotAFunction(func_ty_norm),
                    loc,
                }),
            }
        }
        Expr::Lambda { param_ty, body, .. } => {
            let mut loc0 = loc.clone();
            loc0.push(0);

            let mut loc1 = loc.clone();
            loc1.push(1);

            let r#type = sea.add_node(Expr::Type);
            if infer_type(param_ty, sea, vars_tys, ty_errors, loc0)? != r#type {
                ty_errors.push(TypeErrorWithLoc {
                    error: TypeError::TypeMismatch {
                        expected: r#type,
                        found: param_ty,
                    },
                    loc,
                })
            }

            vars_tys.push(param_ty);
            let ret_ty = infer_type(body, sea, vars_tys, ty_errors, loc1)?;
            let ret_ty_norm = normalize(ret_ty, sea);
            Ok(sea.add_node(Expr::Pi {
                name: (),
                param_ty: param_ty,
                ret_ty: ret_ty_norm,
            }))
        }
        Expr::Pi {
            param_ty, ret_ty, ..
        } => {
            let mut loc0 = loc.clone();
            loc0.push(0);

            let mut loc1 = loc.clone();
            loc1.push(1);

            let r#type = sea.add_node(Expr::Type);
            if infer_type(param_ty, sea, vars_tys, ty_errors, loc0.clone())? != r#type {
                ty_errors.push(TypeErrorWithLoc {
                    error: TypeError::TypeMismatch {
                        expected: r#type,
                        found: param_ty,
                    },
                    loc: loc0,
                })
            }

            if infer_type(ret_ty, sea, vars_tys, ty_errors, loc1.clone())? != r#type {
                ty_errors.push(TypeErrorWithLoc {
                    error: TypeError::TypeMismatch {
                        expected: r#type,
                        found: param_ty,
                    },
                    loc: loc1,
                })
            }

            Ok(r#type)
        }
        Expr::Type => {
            let r#type = sea.add_node(Expr::Type);
            Ok(r#type)
        }
    }?;
    Ok(ty)
}
