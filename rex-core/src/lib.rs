// T should be a wrapper around Expr like F<Expr>
// this is incredibly important for a streaming parser
// instead of going text -> tokens -> sugar_ast -> hash cons
// and building the full structure at each step we can also choose to stream instead. This
// massively improve memory usage and cache locality
// Var is stored as Debruijn indices.
// TODO: Explore removing Var from the enum and instead make T be Either<Box<Expr<..>, Var> this
// way vars are not standalone expressions
//
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Expr<T> {
    Var { idx: usize },
    App { func: T, arg: T },
    Lambda { param_ty: T, body: T },
    Pi { param_ty: T, ret_ty: T },
    Type,
}
