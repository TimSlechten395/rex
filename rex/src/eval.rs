use rex_core::{Expr, ExprF, GExpr};

use crate::sea_nodes::{ExprId, SeaOfNodes};

// what is cutoff?
fn shift(expr: Expr, delta: isize, cutoff: usize) -> Expr {
    let expr = match expr.0.clone() {
        ExprF::Var { idx } => {
            if idx >= cutoff {
                // k + d
                let k_i = idx as isize + delta;
                assert!(k_i >= 0, "shift resulted in negative index");
                ExprF::Var { idx: k_i as usize }
            } else {
                ExprF::Var { idx }
            }
        }
        ExprF::Lambda { param_ty, body, .. } => {
            let param_ty = Box::new(shift(*param_ty, delta, cutoff));
            let body = Box::new(shift(*body, delta, cutoff + 1));
            ExprF::Lambda {
                name: (),
                param_ty,
                body,
            }
        }
        ExprF::Pi {
            param_ty, ret_ty, ..
        } => {
            let param_ty = Box::new(shift(*param_ty, delta, cutoff));
            let ret_ty = Box::new(shift(*ret_ty, delta, cutoff + 1));
            ExprF::Pi {
                name: (),
                param_ty,
                ret_ty,
            }
        }
        ExprF::App { func, arg } => {
            let func = Box::new(shift(*func, delta, cutoff));
            let arg = Box::new(shift(*arg, delta, cutoff));
            ExprF::App { func, arg }
        }
        other => other,
    };
    GExpr(expr)
}

// do we need the index here?
fn substitute(index: usize, func: Expr, arg: Expr) -> Expr {
    let expr = match func.0 {
        ExprF::Var { idx } => {
            if idx == index {
                arg.0
            } else {
                func.0
            }
        }
        //
        ExprF::Lambda { param_ty, body, .. } => {
            // The only important part of this function
            let arg_shifted = shift(arg, 1, 0);
            let body = Box::new(substitute(index + 1, *body, arg_shifted));
            ExprF::Lambda {
                name: (),
                param_ty,
                body,
            }
            // This might be a problem
        }
        ExprF::App {
            func: app_func,
            arg: app_arg,
        } => {
            let app_func = Box::new(substitute(index, *app_func, arg.clone()));
            let app_arg = Box::new(substitute(index, *app_arg, arg));
            ExprF::App {
                func: app_func,
                arg: app_arg,
            }
        }
        ExprF::Pi {
            param_ty, ret_ty, ..
        } => {
            let param_ty = Box::new(substitute(index, *param_ty, arg.clone()));

            let arg_shifted = shift(arg, 1, 0);
            let ret_ty = Box::new(substitute(index, *ret_ty, arg_shifted));

            ExprF::Pi {
                name: (),
                param_ty,
                ret_ty,
            }
        }
        func => func,
    };
    GExpr(expr)
}

pub fn weak_head_normal_form(expr: Expr) -> Expr {
    match expr.0 {
        ExprF::App { func, arg } => {
            let func_eval = weak_head_normal_form(*func);

            match func_eval.0 {
                ExprF::Lambda { body, .. } => {
                    let result = substitute(0, *body, *arg);
                    weak_head_normal_form(result)
                }
                _ => GExpr(ExprF::App {
                    func: Box::new(func_eval),
                    arg,
                }),
            }
        }
        other => GExpr(other),
    }
}

pub fn head_normal_form(expr: Expr) -> Expr {
    match expr.0 {
        ExprF::Lambda { param_ty, body, .. } => {
            let param_ty = Box::new(head_normal_form(*param_ty));
            let body = Box::new(head_normal_form(*body));
            GExpr(ExprF::Lambda {
                name: (),
                param_ty,
                body,
            })
        }
        ExprF::Pi {
            param_ty, ret_ty, ..
        } => {
            let param_ty = Box::new(head_normal_form(*param_ty));
            let ret_ty = Box::new(head_normal_form(*ret_ty));
            GExpr(ExprF::Pi {
                name: (),
                param_ty,
                ret_ty,
            })
        }
        ExprF::App { func, arg } => {
            let func_eval = head_normal_form(*func);

            match func_eval.0 {
                ExprF::Lambda { body, .. } => {
                    let result = substitute(0, *body, *arg);
                    head_normal_form(result)
                }
                _ => GExpr(ExprF::App {
                    func: Box::new(func_eval),
                    arg,
                }),
            }
        }
        _ => expr,
    }
}

pub fn normal_form(expr: Expr) -> Expr {
    match expr.0 {
        ExprF::Lambda { param_ty, body, .. } => {
            let param_ty = Box::new(normal_form(*param_ty));
            let body = Box::new(normal_form(*body));
            GExpr(ExprF::Lambda {
                name: (),
                param_ty,
                body,
            })
        }
        ExprF::Pi {
            param_ty, ret_ty, ..
        } => {
            let param_ty = Box::new(normal_form(*param_ty));
            let ret_ty = Box::new(normal_form(*ret_ty));
            GExpr(ExprF::Pi {
                name: (),
                param_ty,
                ret_ty,
            })
        }
        ExprF::App { func, arg } => {
            let func = normal_form(*func);
            match func.0 {
                ExprF::Lambda { body, .. } => normal_form(substitute(0, *body, *arg)),
                _ => GExpr(ExprF::App {
                    func: Box::new(func),
                    arg: Box::new(normal_form(*arg)),
                }),
            }
        }
        _ => expr,
    }
}
