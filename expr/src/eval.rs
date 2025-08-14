use crate::{Expr, desugar::ExprTree};
use std::collections::HashMap;

// fn shift(k: usize, c: usize, term: &mut Expr) {
//     match term {
//         Expr::Var(i) => {
//             if *i >= k {
//                 *i += c;
//             }
//         }
//         Expr::App(func, arg) => {
//             shift(k, c, func);
//             shift(k, c, arg);
//         }
//         Expr::Lambda(param_ty, body) => {
//             shift(k, c, param_ty);
//             shift(k + 1, c, body); // Variables in the body are under a new binder
//         }
//         Expr::Pi(param_ty, return_ty) => {
//             shift(k, c, param_ty);
//             shift(k + 1, c, return_ty); // Variables in the return type are under a new binder
//         }
//         _ => {}
//     }
// }

// // I am not sure about the De Bruijn index approach it feels like you need a whole pass for one
// // substitution. Stable ids could potentially reduces this to a single pass
// pub fn substitute(idx: usize, sub_expr: &Expr, target_expr: &mut Expr) {
//     match target_expr {
//         Expr::Var(i) => {
//             if *i == idx {
//                 *target_expr = sub_expr.clone();
//                 shift(0, idx, target_expr);
//             } else if *i > idx {
//                 *i -= 1;
//             }
//         }
//         Expr::App(func, arg) => {
//             substitute(idx, sub_expr, func);
//             substitute(idx, sub_expr, arg);
//         }
//         Expr::Lambda(param_ty, body) => {
//             substitute(idx, sub_expr, param_ty);
//             substitute(idx + 1, sub_expr, param_ty);
//         }
//         Expr::Pi(param_ty, body) => {
//             substitute(idx, sub_expr, param_ty);
//             substitute(idx + 1, sub_expr, param_ty);
//         }
//         _ => {}
//     }
// }

// beta reduction. With graphs we only would have to change one node out with the target_expr
pub fn substitute(idx: usize, sub_expr: &ExprTree, target_expr: &mut ExprTree) {
    match &mut **target_expr {
        Expr::Var { idx: i } => {
            if *i == idx {
                *target_expr = sub_expr.clone();
            }
        }
        Expr::App { func, arg } => {
            substitute(idx, sub_expr, &mut **arg);
            substitute(idx, sub_expr, &mut **func);
        }
        Expr::Lambda { body, .. } => {
            substitute(idx, sub_expr, &mut **body);
        }
        Expr::Pi {
            param_ty, ret_ty, ..
        } => {
            substitute(idx, sub_expr, &mut **param_ty);
            substitute(idx, sub_expr, &mut **ret_ty);
        }
        _ => {}
    }
}

pub fn normalize(expr: &mut ExprTree) {
    match &mut **expr {
        Expr::App { func, arg } => {
            normalize(&mut **func);
            normalize(&mut **arg);

            // beta reduction better known as filling in params
            if let Expr::Lambda { body, .. } = &**func.as_mut() {
                let mut new_expr = (**body).clone();
                substitute(0, arg, &mut new_expr);
                *expr = new_expr;
                normalize(expr);
            }

            // BuiltinOp::Type => match arg {
            //     Expr::App(value, r#type) => type_check(eval(*value), eval(*r#type)),
            //     other => {
            //         Expr::App(Box::new(Expr::Builtin(BuiltinOp::Type)), Box::new(other))
            //     }
            // },
        }
        Expr::Lambda { body, .. } => {
            normalize(&mut **body);
        }

        Expr::Pi {
            param_ty, ret_ty, ..
        } => {
            normalize(&mut **param_ty);
            normalize(&mut **ret_ty);
        }
        Expr::Ann { expr, ty } => {
            normalize(&mut **expr);
            normalize(&mut **ty);
        }
        _ => {}
    }
}
