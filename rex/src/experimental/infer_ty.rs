// #[derive(Clone)]
// pub enum Expr {
//     Lam(Box<Expr>),
//     App(Box<Expr>, Box<Expr>),
//     Var(usize),
// }
//
// #[derive(Clone)]
// pub enum Type {
//     Pi(Box<Type>, Box<Type>),
//     Type,
//     Var(usize),
// }
//
// #[derive(Clone)]
// pub enum FullExpr {
//     Expr(Expr)
//     Pi(Box<Type>, Box<Type>),
//     Type,
//     Var(usize),
// }
//
// type Context = Vec<Type>;
//
// pub fn infer_type(expr: Expr, context: Context) -> Option<Type> {
//     match expr {
//         Expr::Var(i) => return context.get(i).cloned(),
//         Expr::Lam(body) => {
//             let x = Type::Var(1);
//         }
//         Expr::App(f, e) => {
//             let tf = infer_type(f, context);
//             let te = infer_type(f, context);
//
//         }
//     };
// }
