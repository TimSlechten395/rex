use std::{
    collections::{HashMap, hash_map::Entry},
    hash::{DefaultHasher, Hash, Hasher},
};

use thiserror::Error;

use crate::{Expr, ExprTree};

// We need hashes if we want parallell compilation. different cores need to end up with the same
// hash for equal expressions
pub type NodeId = u64;

pub type ExprGraph = Expr<NodeId, usize, ()>;

// invariant: id = reverse.get(nodes.get(id))
// TODO: figure out of reference counting is usefull here
// NOTE: We could cache this for incremental compilation but we need a way to get rid of dead code
// which means we need reference counting. We could store an extra HashMap<NodeId, Count> or we
// could use Rc<ExprGraph>
// WARN: neutral forms interact weirdly with the cache for example lambda 0 3 here three is a free
// variable but for free variable it does not matter the index of the variable for application it
// is seen as unknown
#[derive(Debug, Clone)]
pub struct SeaOfNodes {
    // usize is reference count
    pub nodes: HashMap<NodeId, (ExprGraph, usize)>,
    // this map was needed for sequential ids but now we just hash ExprGraph directly
    // reverse: HashMap<ExprGraph, NodeId>,
    // this is a substitution map for example the nodeId of add 5 4 points to the node_id of 9
    pub cache: HashMap<NodeId, NodeId>,
    // Store the type of an expr. for example: 3: Nat and Nat: Type
    // NOTE: types only make sense for combinators
    pub tys: HashMap<NodeId, NodeId>,
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
        node.hash(&mut hasher);
        let hash = hasher.finish();
        match self.nodes.entry(hash) {
            Entry::Vacant(v) => {
                v.insert((node, 1));
            }
            Entry::Occupied(mut o) => o.get_mut().1 += 1,
        }
        hash
    }

    pub fn get_node(&self, id: NodeId) -> Option<&ExprGraph> {
        self.nodes.get(&id).map(|x| &x.0)
    }

    pub fn get_tree(&self, id: NodeId) -> Option<ExprTree<usize, ()>> {
        let graph = self.get_node(id)?;
        let inner = match graph {
            Expr::Var { idx } => Expr::Var { idx: *idx },
            Expr::App { func, arg } => Expr::App {
                func: Box::new(self.get_tree(*func)?),
                arg: Box::new(self.get_tree(*arg)?),
            },
            Expr::Lambda { param_ty, body, .. } => Expr::Lambda {
                name: (),
                param_ty: Box::new(self.get_tree(*param_ty)?),
                body: Box::new(self.get_tree(*body)?),
            },
            Expr::Pi {
                param_ty, ret_ty, ..
            } => Expr::Pi {
                name: (),
                param_ty: Box::new(self.get_tree(*param_ty)?),
                ret_ty: Box::new(self.get_tree(*ret_ty)?),
            },
            Expr::Type => Expr::Type,
        };

        Some(ExprTree(inner))
    }
}

// its a little weird to have nodes for DeBruijn indices but I guess it works
// This function should be a single MapAccumL
// TODO: stupid idea let Expr::Var contain id of the lambda it is bound to.
// That does not work we get the id by hashing. and we need DeBruijn index anyway for
// uniqueness.
// HACK: We are adding variables to the sea of nodes but Var with an index is absolutely not
// unique it just makes it so comparing lambdas is just comparing hashes.
// NOTE: The longer I think about this, it feels like the only correct way to implement. I tried
// unique indices but you never succeed in hashconsing lambdas
// TODO: If somehow we could link it back to the lambda it comes from we can store type information
// next to Vars but I guess the lambda stores Var anyway?
pub fn lower_expr(ast: &ExprTree<usize, ()>, sea: &mut SeaOfNodes) -> NodeId {
    match &ast.0 {
        Expr::Var { idx } => sea.add_node(ExprGraph::Var { idx: *idx }),
        Expr::App { func, arg } => {
            let func_id = lower_expr(func, sea);
            let arg_id = lower_expr(arg, sea);
            sea.add_node(ExprGraph::App {
                func: func_id,
                arg: arg_id,
            })
        }
        Expr::Lambda {
            name: _,
            param_ty,
            body,
        } => {
            let param_ty = lower_expr(param_ty, sea);
            let body = lower_expr(body, sea);
            sea.add_node(ExprGraph::Lambda {
                name: (),
                param_ty,
                body,
            })
        }
        Expr::Pi {
            name: _,
            param_ty,
            ret_ty,
        } => {
            let param_ty_id = lower_expr(param_ty, sea);
            let ret_ty_id = lower_expr(ret_ty, sea);
            sea.add_node(Expr::Pi {
                name: (),
                param_ty: param_ty_id,
                ret_ty: ret_ty_id,
            })
        }
        Expr::Type => sea.add_node(Expr::Type),
    }
}
