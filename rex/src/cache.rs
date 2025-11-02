use std::{
    collections::{HashMap, hash_map::Entry},
    hash::{DefaultHasher, Hash, Hasher},
};

use crate::data::expr::{Expr, ExprF, GExpr};

// hash for equal expressions
pub type ExprId = u64;

pub type ExprGraph = ExprF<ExprId, usize, ()>;

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
    pub nodes: HashMap<ExprId, (ExprGraph, usize)>,
    // this map was needed for sequential ids but now we just hash ExprGraph directly
    // reverse: HashMap<ExprGraph, NodeId>,
    // this is a substitution map for example the nodeId of add 5 4 points to the node_id of 9
    pub cache: HashMap<ExprId, ExprId>,
    // Store the type of an expr. for example: 3: Nat and Nat: Type
    // NOTE: types only make sense for combinators
    pub tys: HashMap<ExprId, ExprId>,
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
    pub fn add_node(&mut self, node: ExprGraph) -> ExprId {
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

    pub fn get_node(&self, id: ExprId) -> Option<&ExprGraph> {
        self.nodes.get(&id).map(|x| &x.0)
    }

    pub fn get_tree(&self, id: ExprId) -> Option<Expr> {
        let graph = self.get_node(id)?;
        let inner = match graph {
            ExprF::Var { idx } => ExprF::Var { idx: *idx },
            ExprF::App { func, arg } => ExprF::App {
                func: Box::new(self.get_tree(*func)?),
                arg: Box::new(self.get_tree(*arg)?),
            },
            ExprF::Lambda { param_ty, body, .. } => ExprF::Lambda {
                name: (),
                param_ty: Box::new(self.get_tree(*param_ty)?),
                body: Box::new(self.get_tree(*body)?),
            },
            ExprF::Pi {
                param_ty, ret_ty, ..
            } => ExprF::Pi {
                name: (),
                param_ty: Box::new(self.get_tree(*param_ty)?),
                ret_ty: Box::new(self.get_tree(*ret_ty)?),
            },
            ExprF::Type => ExprF::Type,
        };

        Some(GExpr(inner))
    }
}

pub fn lower_expr(ast: &Expr, sea: &mut SeaOfNodes) -> ExprId {
    match &ast.0 {
        ExprF::Var { idx } => sea.add_node(ExprGraph::Var { idx: *idx }),
        ExprF::App { func, arg } => {
            let func_id = lower_expr(func, sea);
            let arg_id = lower_expr(arg, sea);
            sea.add_node(ExprGraph::App {
                func: func_id,
                arg: arg_id,
            })
        }
        ExprF::Lambda {
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
        ExprF::Pi {
            name: _,
            param_ty,
            ret_ty,
        } => {
            let param_ty_id = lower_expr(param_ty, sea);
            let ret_ty_id = lower_expr(ret_ty, sea);
            sea.add_node(ExprF::Pi {
                name: (),
                param_ty: param_ty_id,
                ret_ty: ret_ty_id,
            })
        }
        ExprF::Type => sea.add_node(ExprF::Type),
    }
}
