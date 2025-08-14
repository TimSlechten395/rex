use std::collections::HashMap;

use crate::{Expr, ExprTree};

pub type NodeId = usize;

pub type ExprGraph = Expr<NodeId>;

// invariant: id = reverse.get(nodes.get(id))
// TODO: figure out of reference counting is usefull here
pub struct SeaOfNodes {
    nodes: HashMap<NodeId, ExprGraph>,
    reverse: HashMap<ExprGraph, NodeId>,
    // this is a substitution map for example the nodeId of add 5 4 points to the node_id of 9
    forward: HashMap<NodeId, NodeId>,
    next_id: NodeId,
}

impl SeaOfNodes {
    pub fn new() -> Self {
        SeaOfNodes {
            nodes: HashMap::new(),
            reverse: HashMap::new(),
            forward: HashMap::new(),
            next_id: 0,
        }
    }

    pub fn add_node(&mut self, node: ExprGraph) -> NodeId {
        if let Some(&id) = self.reverse.get(&node) {
            return id;
        }
        let id = self.next_id;
        self.next_id += 1;
        self.nodes.insert(id, node.clone());
        self.reverse.insert(node, id);
        id
    }

    pub fn get_node(&self, id: NodeId) -> Option<&ExprGraph> {
        self.nodes.get(&id)
    }
}

// its a little weird to have nodes for DeBruijn idices but I guess it works
// This function should be a single MapAccumL
fn lower_expr(ast: &ExprTree, sea: &mut SeaOfNodes) -> NodeId {
    match &**ast {
        Expr::Hole => sea.add_node(ExprGraph::Hole),
        Expr::Var { idx } => sea.add_node(ExprGraph::Var { idx }),
        Expr::App { func, arg } => {
            let func_id = lower_expr(func, sea);
            let arg_id = lower_expr(arg, sea);
            sea.add_node(ExprGraph::App {
                func: func_id,
                arg: arg_id,
            })
        }
        Expr::Lambda { body } => {
            let body_id = lower_expr(body, sea);
            sea.add_node(ExprGraph::Lambda { body: &**body })
        }
        Expr::Pi { param_ty, ret_ty } => {
            let param_ty_id = lower_expr(param_ty, sea);
            let ret_ty_id = lower_expr(ret_ty, sea);
            sea.add_node(Expr::Pi {
                param_ty: param_ty_id,
                ret_ty: ret_ty_id,
            })
        }
        Expr::Type => sea.add_node(Expr::Type),
        Expr::Ann { expr, ty } => {
            let expr_id = lower_expr(expr, sea);
            let ty_id = lower_expr(ty, sea);
            sea.add_node(Expr::Ann {
                expr: expr_id,
                ty: ty_id,
            })
        }
    }
}

// what is cutoff?
fn shift(expr: NodeId, delta: isize, cutoff: usize, sea: &mut SeaOfNodes) -> NodeId {
    let expr = sea.get_node(expr).unwrap();
    let expr = match expr {
        Expr::Var(k) => {
            if *k >= cutoff {
                // k + d
                let k_i = *k as isize + delta;
                assert!(k_i >= 0, "shift resulted in negative index");
                Expr::Var(k_i as usize)
            } else {
                Expr::Var(*k)
            }
        }
        Expr::Lambda(body) => {
            let body = shift(body, delta, cutoff + 1, sea);
            Expr::Lambda { body }
        }
        Expr::Pi { param_ty, ret_ty } => {
            let param_ty = shift(ret_ty, delta, cutoff, sea);
            let ret_ty = shift(ret_ty, delta, cutoff + 1, sea);
            Expr::Pi { param_ty, ret_ty }
        }
        Expr::App(func, arg) => {
            let func = shift(func, delta, cutoff, sea);
            let arg = shift(arg, delta, cutoff, sea);
            Expr::App { func, arg }
        }
        Expr::Ann { expr, ty } => {
            let expr = shift(expr, delta, cutoff, sea);
            let ty = shift(ty, delta, cutoff, sea);
            Expr::Ann { expr, ty }
        }
        _ => (),
    };
    sea.add_node(expr)
}

// take an function an and argument apply and return the new node id.
// for now we only substiture directly so no shifts need to happen.
// unwraps can never fail here.
// A special fold function would be nice here as well.
fn substitute(index: usize, func: NodeId, arg: NodeId, sea: &mut SeaOfNodes) -> NodeId {
    let node = sea.get_node(func).unwrap();
    match node {
        Expr::Var { idx } => {
            if *idx == index {
                arg
            } else {
                Expr::Var { idx }
            }
        }
        //
        Expr::Lambda { body } => {
            // The only important part of this function
            let arg_shifted = shift(arg, 1, 0, sea);
            let body = substitute(index + 1, body, arg_shifted, sea);
            let new = Expr::Lambda { body };
            // This might be a problem
            sea.add_node(new)
        }
        Expr::App {
            func: app_func,
            arg: app_arg,
        } => {
            let app_func = substitute(index, *app_func, arg, sea);
            let app_arg = substitute(index, *app_arg, arg, sea);
            let new = Expr::App {
                func: app_func,
                arg: app_arg,
            };
            sea.add_node(new)
        }
        Expr::Pi { param_ty, ret_ty } => {
            let param_ty = substitute(index, *param_ty, arg, sea);

            let arg_shifted = shift(arg, 1, 0, sea);
            let ret_ty = substitute(index, *ret_ty, arg_shifted, sea);

            let new = Expr::Pi { param_ty, ret_ty };
            sea.add_node(new)
        }
        Expr::Ann { expr, ty } => {
            let param_ty = substitute(index, *expr, arg, sea);
            let ret_ty = substitute(index, *ty, arg, sea);
            let new = Expr::Ann { expr, ty };
            sea.add_node(new)
        }
        _ => node,
    }
}

// weak normalize make sure there is no application at the end;
pub fn normalize(expr: NodeId, sea: &mut SeaOfNodes) -> NodeId {
    let expr = sea.get_node(expr).unwrap();
    let new_expr = if let Some(&Expr::App { func, arg }) = expr {
        let func = normalize(func, sea);
        substitute(0, func, arg, sea)
    } else {
        expr
    };
    sea.add_node(new_expr)
}

pub fn eager_normalize(expr: NodeId, sea: &mut SeaOfNodes) -> NodeId {
    let expr = sea.get_node(expr).unwrap();
    let new_expr = if let Some(&Expr::App { func, arg }) = expr {
        let func = normalize(func, sea);
        let arg = normalize(arg, sea);
        let new_expr = substitute(0, func, arg, sea);
    } else {
        expr
    };
    sea.add_node(new_expr)
}

// normalize everything as agressively as possible
pub fn strong_normalize(expr: NodeId, sea: &mut SeaOfNodes) -> NodeId {
    let expr = sea.get_node(expr);
    let new_expr = match expr {
        Expr::App { func, arg } => {
            let func = strong_normalize(func, sea);
            let arg = strong_normalize(arg, sea);
            substitute(0, func, arg, sea)
        }
        Expr::Lambda { body } => strong_normalize(body, sea),
        Expr::Pi { param_ty, ret_ty } => {
            let param_ty = strong_normalize(param_ty, sea);
            let ret_ty = strong_normalize(ret_ty, sea);
            Expr::Pi { param_ty, ret_ty }
        }
        Expr::Ann { expr, ty } => {
            strong_normalize(expr, sea);
            strong_normalize(ty, sea);
        }
    };
    let new_expr = sea.add_node(new_expr);
    sea.add_node(new_expr)
    // sea.forward.insert(expr, new_expr);
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SeaExpr {
    App { func: NodeId, arg: NodeId },
    Lambda { body: Expr<NodeId> },
    // first expr is type second is body
    Pi { param_ty: NodeId, ret_ty: NodeId },
    Type,
    Ann { expr: NodeId, ty: NodeId },
    // represent _ in type meaning to be ignored unification means this variant always loses
    Hole,
}
