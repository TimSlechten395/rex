use std::{
    collections::{HashMap, hash_map::Entry},
    hash::{DefaultHasher, Hash, Hasher},
};

use crate::{Expr, ExprTree};

// We need hashes if we want parallell compilation. different cores need to end up with the same
// hash for equal expressions
pub type NodeId = u64;

pub type ExprGraph = Expr<NodeId>;

// invariant: id = reverse.get(nodes.get(id))
// TODO: figure out of reference counting is usefull here
// NOTE: We could cache this for incremental compilation but we need a way to get rid of dead code
// which means we need reference counting
#[derive(Debug, Clone)]
pub struct SeaOfNodes {
    // could be HashMap<NodeId, Rc<ExprGraph>> but this does not make jfj
    nodes: HashMap<NodeId, ExprGraph>,
    // this map was needed for sequential ids but now we just hash ExprGraph directly
    // reverse: HashMap<ExprGraph, NodeId>,
    // this is a substitution map for example the nodeId of add 5 4 points to the node_id of 9
    cache: HashMap<NodeId, NodeId>,
    // Store the type of an expr. for example: 3: Nat and Nat: Type
    tys: HashMap<NodeId, NodeId>,
}

impl SeaOfNodes {
    pub fn new() -> Self {
        SeaOfNodes {
            nodes: HashMap::new(),
            // reverse: HashMap::new(),
            cache: HashMap::new(),
            tys: HashMap::new(),
        }
    }

    // WARN: Surely hash collisions cannot happen
    // Technically we can avoid collisions by holding a HashMap<NodeId, SmallVec<ExprGraph>> instead but
    // this is quite expensive
    pub fn add_node(&mut self, node: ExprGraph) -> NodeId {
        let mut hasher = DefaultHasher::new();
        let node_id = node.hash(&mut hasher);
        let hash = hasher.finish();
        match self.nodes.entry(hash) {
            Entry::Vacant(v) => {
                v.insert(node);
            }
            Entry::Occupied(_) => (),
        }
        hash
    }

    pub fn get_node(&self, id: NodeId) -> Option<&ExprGraph> {
        self.nodes.get(&id)
    }
}

// its a little weird to have nodes for DeBruijn indices but I guess it works
// This function should be a single MapAccumL
// TODO: stupid idea let Expr::Var contain id of the lambda it is bound to.
// That does not work we get the id by hashing. and we need DeBruijn index anyway for
// uniqueness.
fn lower_expr(ast: &ExprTree, sea: &mut SeaOfNodes) -> NodeId {
    match &**ast {
        Expr::Var { idx } => sea.add_node(ExprGraph::Var { idx: *idx }),
        Expr::App { func, arg } => {
            let func_id = lower_expr(func, sea);
            let arg_id = lower_expr(arg, sea);
            sea.add_node(ExprGraph::App {
                func: func_id,
                arg: arg_id,
            })
        }
        Expr::Lambda { param_ty, body } => {
            let param_ty = lower_expr(param_ty, sea);
            let body = lower_expr(body, sea);
            sea.add_node(ExprGraph::Lambda { param_ty, body })
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
    }
}

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
        Expr::Lambda { param_ty, body } => {
            let param_ty = shift(param_ty, delta, cutoff, sea);
            let body = shift(body, delta, cutoff + 1, sea);
            Expr::Lambda { param_ty, body }
        }
        Expr::Pi { param_ty, ret_ty } => {
            let param_ty = shift(param_ty, delta, cutoff, sea);
            let ret_ty = shift(ret_ty, delta, cutoff + 1, sea);
            Expr::Pi { param_ty, ret_ty }
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
        Expr::Lambda { param_ty, body } => {
            // The only important part of this function
            let arg_shifted = shift(arg, 1, 0, sea);
            let body = substitute(index + 1, body, arg_shifted, sea);
            let new = Expr::Lambda { param_ty, body };
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
        Expr::Pi { param_ty, ret_ty } => {
            let param_ty = substitute(index, param_ty, arg, sea);

            let arg_shifted = shift(arg, 1, 0, sea);
            let ret_ty = substitute(index, ret_ty, arg_shifted, sea);

            let new = Expr::Pi { param_ty, ret_ty };
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
            Expr::Lambda { param_ty, body } => strong_normalize(body, sea),
            Expr::Pi { param_ty, ret_ty } => {
                let param_ty = strong_normalize(param_ty, sea);
                let ret_ty = strong_normalize(ret_ty, sea);
                let new_expr = Expr::Pi { param_ty, ret_ty };

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

#[derive(Debug)]
pub enum TypeError {
    UnboundVariable(usize),
    NotAFunction(NodeId),
    NotAType(NodeId),
    TypeMismatch { expected: NodeId, found: NodeId },
}

// Return the type for a term
// Unfortunalty we need vars_tys to resolve tys for the vars. If vars were globally unique this
// would not be a problem
// TODO: check if is is possible to use a generic Expr<T> instead. I assume it is impossible
// because there is no generic way to traverse the expression.
pub fn infer_type(
    expr: NodeId,
    sea: &mut SeaOfNodes,

    vars_tys: &mut Vec<NodeId>,
) -> Result<NodeId, TypeError> {
    let expr = sea.get_node(expr).unwrap().clone();
    match expr {
        // do NOT include this is the TypeContext as
        Expr::Var { idx } => {
            if let Some(ty) = vars_tys.iter().rev().nth(idx) {
                Ok(*ty)
            } else {
                Err(TypeError::UnboundVariable(idx))
            }
        }

        // get the function param type and the arg type see if they are the same and then return
        // the function return type
        Expr::App { func, arg } => {
            let func_ty = infer_type(func, sea, vars_tys)?;
            let func_ty_norm = normalize(func_ty, sea);
            let func_ty_norm = sea.get_node(func_ty_norm).unwrap().clone();

            if let Expr::Pi { param_ty, ret_ty } = func_ty_norm {
                let param_ty_norm = normalize(param_ty, sea);
                let arg_ty = infer_type(arg, sea, vars_tys);
                let arg_ty_norm = normalize(arg, sea);

                // check if types are the same
                if param_ty_norm == arg_ty_norm {
                    let ret_ty_norm = normalize(ret_ty, sea);
                    Ok(ret_ty_norm)
                } else {
                    Err(TypeError::TypeMismatch {
                        expected: param_ty_norm,
                        found: arg_ty_norm,
                    })
                }
            } else {
                Err(TypeError::NotAFunction(func_ty))
            }
        }
        Expr::Lambda { param_ty, body } => {
            check_is_type(param_ty, sea, vars_tys)?;
            vars_tys.push(param_ty);
            let ret_ty = infer_type(body, sea, vars_tys)?;
            Ok(sea.add_node(Expr::Pi {
                param_ty: param_ty,
                ret_ty,
            }))
        }
        // the resulting type is always Expr Type but we need to check if the Pi is well formed
        Expr::Pi { param_ty, ret_ty } => {
            check_is_type(param_ty, sea, vars_tys)?;
            check_is_type(ret_ty, sea, vars_tys)?;
            let r#type = sea.add_node(Expr::Type);
            Ok(r#type)
        }
        Expr::Type => {
            let r#type = sea.add_node(Expr::Type);
            Ok(r#type)
        }
    }
}
pub fn check_is_type(
    expr: NodeId,
    sea: &mut SeaOfNodes,
    vars_tys: &mut Vec<NodeId>,
) -> Result<(), TypeError> {
    let infer_ty = infer_type(expr, sea, vars_tys)?;
    let infer_ty_norm = normalize(infer_ty, sea);
    let infer_ty_norm = sea.get_node(infer_ty_norm).unwrap();

    if let Expr::Type = infer_ty_norm {
        Ok(())
    } else {
        Err(TypeError::NotAType(infer_ty))
    }
}
