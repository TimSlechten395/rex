use rex_core::Expr;

use crate::sea_nodes::{NodeId, SeaOfNodes};

// what is cutoff?
fn shift(expr_id: NodeId, delta: isize, cutoff: usize, sea: &mut SeaOfNodes) -> NodeId {
    let expr = sea.get_node(expr_id).unwrap();
    let expr = match expr.clone() {
        Expr::Var { idx } => {
            if idx >= cutoff {
                // k + d
                let k_i = idx as isize + delta;
                assert!(k_i >= 0, "shift resulted in negative index");
                Expr::Var { idx: k_i as usize }
            } else {
                Expr::Var { idx }
            }
        }
        Expr::Lambda { param_ty, body, .. } => {
            let param_ty = shift(param_ty, delta, cutoff, sea);
            let body = shift(body, delta, cutoff + 1, sea);
            Expr::Lambda {
                name: (),
                param_ty,
                body,
            }
        }
        Expr::Pi {
            param_ty, ret_ty, ..
        } => {
            let param_ty = shift(param_ty, delta, cutoff, sea);
            let ret_ty = shift(ret_ty, delta, cutoff + 1, sea);
            Expr::Pi {
                name: (),
                param_ty,
                ret_ty,
            }
        }
        Expr::App { func, arg } => {
            let func = shift(func, delta, cutoff, sea);
            let arg = shift(arg, delta, cutoff, sea);
            Expr::App { func, arg }
        }
        other => other,
    };
    sea.add_node(expr)
}

// take an function an and argument apply and return the new node id.
// for now we only substiture directly so no shifts need to happen.
// unwraps can never fail here.
// A special fold function would be nice here as well.
fn substitute(index: usize, func: NodeId, arg: NodeId, sea: &mut SeaOfNodes) -> NodeId {
    let node = sea.get_node(func).unwrap();
    match *node {
        Expr::Var { idx } => {
            if idx == index {
                arg
            } else {
                func
            }
        }
        //
        Expr::Lambda { param_ty, body, .. } => {
            // The only important part of this function
            let arg_shifted = shift(arg, 1, 0, sea);
            let body = substitute(index + 1, body, arg_shifted, sea);
            let new = Expr::Lambda {
                name: (),
                param_ty,
                body,
            };
            // This might be a problem
            sea.add_node(new)
        }
        Expr::App {
            func: app_func,
            arg: app_arg,
        } => {
            let app_func = substitute(index, app_func, arg, sea);
            let app_arg = substitute(index, app_arg, arg, sea);
            let new = Expr::App {
                func: app_func,
                arg: app_arg,
            };
            sea.add_node(new)
        }
        Expr::Pi {
            param_ty, ret_ty, ..
        } => {
            let param_ty = substitute(index, param_ty, arg, sea);

            let arg_shifted = shift(arg, 1, 0, sea);
            let ret_ty = substitute(index, ret_ty, arg_shifted, sea);

            let new = Expr::Pi {
                name: (),
                param_ty,
                ret_ty,
            };
            sea.add_node(new)
        }
        _ => func,
    }
}

// TODO: Should we pass in expr_id or generic Expr<T>
//
// weak normalize make sure there is no application at the end
pub fn normalize(expr_id: NodeId, sea: &mut SeaOfNodes) -> NodeId {
    let expr = sea.get_node(expr_id).unwrap();

    if let &Expr::App { func, arg } = expr {
        let func = normalize(func, sea);
        substitute(0, func, arg, sea)
    } else {
        expr_id
    }
}

// This is normal execution. basically a small interpreter
pub fn eager_normalize(expr_id: NodeId, sea: &mut SeaOfNodes) -> NodeId {
    let expr = sea.get_node(expr_id).unwrap();
    if let &Expr::App { func, arg } = expr {
        let func = normalize(func, sea);
        let arg = normalize(arg, sea);
        let new_expr = substitute(0, func, arg, sea);
        new_expr
    } else {
        expr_id
    }
}

// normalize everything, even inside lambdas.
// TODO: This functions does to much. Reading and writing the cache should be done at a
// higher level
pub fn strong_normalize(expr: NodeId, sea: &mut SeaOfNodes) -> NodeId {
    let new_expr = if let Some(new_expr) = sea.cache.get(&expr).cloned() {
        new_expr
    } else {
        let expr_node = sea.get_node(expr).unwrap();
        let new_expr = match *expr_node {
            Expr::App { func, arg } => {
                let func = strong_normalize(func, sea);
                let arg = strong_normalize(arg, sea);
                substitute(0, func, arg, sea)
            }
            Expr::Lambda { param_ty, body, .. } => strong_normalize(body, sea),
            Expr::Pi {
                param_ty, ret_ty, ..
            } => {
                let param_ty = strong_normalize(param_ty, sea);
                let ret_ty = strong_normalize(ret_ty, sea);
                let new_expr = Expr::Pi {
                    name: (),
                    param_ty,
                    ret_ty,
                };

                sea.add_node(new_expr)
            }
            _ => expr,
        };
        new_expr
    };
    // Do we need to do this
    sea.cache.insert(expr, new_expr);
    new_expr
}
