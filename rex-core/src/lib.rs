// T should be a wrapper around Expr like F<Expr>
// our compiler goes from text -> tokens -> sugar_ast -> typed_sugar_ast -> core
// Var is stored as Debruijn indices.
// TODO: explore using globally unique ids for vars, or storing the type directly with the var
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Expr<T> {
    Var { idx: usize },
    App { func: T, arg: T },
    Lambda { param_ty: T, body: T },
    Pi { param_ty: T, ret_ty: T },
    Type,
}
