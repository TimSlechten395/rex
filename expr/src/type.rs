use std::collections::HashMap;

use crate::{Expr, ExprTree, normalize, substitute};

// TODO: somewhere in here we need a recursion handler

// Variables are represented as globally unique indices. This mean this type context never has to
// reset
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TypeContext {
    pub entries: HashMap<usize, ExprTree>,
}

#[derive(Debug)]
pub enum TypeError {
    UnboundVariable(usize),
    NotAFunction(ExprTree),
    NotAType(ExprTree),
    TypeMismatch { expected: ExprTree, found: ExprTree },
}

// Return the type for a term
// pub fn infer_type(term: &ExprTree, ctx: &mut TypeContext) -> Result<ExprTree, TypeError> {
//     let expr = match &**term {
//         Expr::Var { idx: var_id } => ctx
//             .entries
//             .get(var_id)
//             .cloned()
//             .ok_or(TypeError::UnboundVariable(*var_id)),
//
//         // get the function param type and the arg type see if they are the same and then return
//         // the function return type
//         Expr::App { func, arg } => {
//             let func_ty = infer_type(func, ctx)?;
//             let mut func_ty_norm = func_ty.clone();
//             normalize(&mut func_ty_norm);
//
//             if let Expr::Pi { param_ty, ret_ty } = &**func_ty_norm {
//                 check_type(arg, &param_ty, ctx)?;
//
//                 let mut normalized_arg = (**arg).clone();
//                 normalize(&mut normalized_arg);
//
//                 let mut res_ty = (*ret_ty).clone();
//                 substitute(param_id, &normalized_arg, &mut res_ty);
//                 normalize(&mut res_ty);
//                 Ok(res_ty)
//             } else {
//                 Err(TypeError::NotAFunction(func_ty))
//             }
//         }
//         Expr::Lambda { body } => {
//             check_is_type(param_ty, ctx)?;
//             ctx.entries.insert(*param_id, *param_ty.clone());
//             let ret_ty = infer_type(body, ctx)?;
//             Ok(Expr::Pi {
//                 ret_ty: Box::new(ret_ty),
//             })
//         }
//         // the resulting type is always Expr Type but we need to check if the Pi is well formed
//         Expr::Pi { param_ty, ret_ty } => {
//             check_is_type(param_ty, ctx)?;
//             ctx.entries.insert(*param_id, *param_ty.clone());
//             check_is_type(ret_ty, ctx)?;
//             Ok(Expr::Type)
//         }
//         Expr::Type => Ok(Expr::Type),
//         Expr::Ann { expr, ty } => {
//             check_type(expr, ty, ctx)?;
//             Ok(*ty.clone())
//         }
//         _ => Ok(Expr::Bool(true)),
//     };
//
//     expr.map(ExprTree)
// }
//
// // check the type of an expression and see if it is the same
// pub fn check_type(
//     expr: &ExprTree,
//     exp_ty: &ExprTree,
//     ctx: &mut TypeContext,
// ) -> Result<(), TypeError> {
//     let infer_ty = infer_type(expr, ctx)?;
//     if equality(infer_ty.clone(), exp_ty.clone()) {
//         Ok(())
//     } else {
//         Err(TypeError::TypeMismatch {
//             expected: exp_ty.clone(),
//             found: infer_ty,
//         })
//     }
// }
//
// pub fn check_is_type(expr: &ExprTree, ctx: &mut TypeContext) -> Result<(), TypeError> {
//     let infer_ty = infer_type(expr, ctx)?;
//     let mut infer_ty_norm = infer_ty.clone();
//     normalize(&mut infer_ty_norm);
//
//     if let Expr::Type = &**infer_ty_norm {
//         Ok(())
//     } else {
//         Err(TypeError::NotAType(infer_ty))
//     }
// }
//
// // two Exprs are the same if there normal form is the same
// pub fn equality(mut ty1: ExprTree, mut ty2: ExprTree) -> bool {
//     normalize(&mut ty1);
//     normalize(&mut ty2);
//
//     ty1 == ty2
// }
