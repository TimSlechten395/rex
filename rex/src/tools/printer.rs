use crate::data::expr::{Expr, ExprF, NamedExpr, VarKind};

#[derive(PartialEq, PartialOrd, Debug, Clone, Copy)]
pub enum Prec {
    LOWEST = 0,
    PI = 1,
    LAMBDA = 2,
    APP = 3,
    HIGHEST = 4,
}

#[derive(Debug, Clone, Copy)]
pub enum Assoc {
    Left,
    Right,
}

//
fn maybe_parens(
    s: String,
    my_prec: Prec,
    parent_prec: Prec,
    parent_assoc: Option<Assoc>,
    is_right_child: bool,
) -> String {
    let needs_parens = if my_prec < parent_prec {
        true
    } else if my_prec == parent_prec {
        match (parent_assoc, is_right_child) {
            (Some(Assoc::Left), true) => true,
            (Some(Assoc::Right), false) => true,
            _ => false,
        }
    } else {
        false
    };
    if needs_parens { format!("({})", s) } else { s }
}

pub fn print(code: NamedExpr, prec: Prec, assoc: Option<Assoc>, is_right_child: bool) -> String {
    use ExprF::*;

    match code.0 {
        Var { idx } => match idx {
            VarKind::Named(s) => s,
            VarKind::Idx(i) => i.to_string(),
        },

        Type => "Type".to_string(),

        App { func, arg } => {
            let (my_prec, my_assoc) = (Prec::APP, Some(Assoc::Left));
            let s = format!(
                "{} {}",
                print(*func, my_prec, my_assoc, false),
                print(*arg, my_prec, my_assoc, true)
            );
            maybe_parens(s, my_prec, prec, assoc, is_right_child)
        }

        Lambda {
            name,
            param_ty,
            body,
        } => {
            let (my_prec, my_assoc) = (Prec::LAMBDA, Some(Assoc::Right));
            let param_ty = print(*param_ty, Prec::LAMBDA, my_assoc, false);
            let body = print(*body, my_prec, my_assoc, true);

            let s = match name {
                VarKind::Named(name) => {
                    let param_ty = print(*param_ty, Prec::LOWEST, my_assoc, false);
                    format!("({}: {}) => {}", name, param_ty, body)
                }
                VarKind::Idx(_) => format!("{} => {}", param_ty, body),
            };

            maybe_parens(s, my_prec, prec, assoc, is_right_child)
        }

        Pi {
            name,
            param_ty,
            ret_ty,
        } => {
            let (my_prec, my_assoc) = (Prec::PI, Some(Assoc::Right));
            let param_ty = print(*param_ty, my_prec, my_assoc, false);
            let ret_ty = print(*ret_ty, my_prec, my_assoc, true);

            let s = match name {
                VarKind::Named(name) => {
                    let param_ty = print(*param_ty, Prec::LOWEST, my_assoc, false);
                    format!("({}: {}) -> {}", name, param_ty, ret_ty)
                }
                VarKind::Idx(_) => format!("{} -> {}", param_ty, ret_ty),
            };

            maybe_parens(s, my_prec, prec, assoc, is_right_child)
        }
    }
}

pub fn print_expr(code: Expr, prec: Prec, assoc: Option<Assoc>, is_right_child: bool) -> String {
    use ExprF::*;

    match code.0 {
        Var { idx } => idx.to_string(),

        Type => "Type".to_string(),

        App { func, arg } => {
            let (my_prec, my_assoc) = (Prec::APP, Some(Assoc::Left));
            let s = format!(
                "{} {}",
                print_expr(*func, my_prec, my_assoc, false),
                print_expr(*arg, my_prec, my_assoc, true)
            );
            maybe_parens(s, my_prec, prec, assoc, is_right_child)
        }

        Lambda {
            name,
            param_ty,
            body,
        } => {
            let (my_prec, my_assoc) = (Prec::LAMBDA, Some(Assoc::Right));
            let param_ty = print_expr(*param_ty, my_prec, my_assoc, false);
            let body = print_expr(*body, my_prec, my_assoc, true);

            let s = format!("{} => {}", param_ty, body);

            maybe_parens(s, my_prec, prec, assoc, is_right_child)
        }

        Pi {
            name,
            param_ty,
            ret_ty,
        } => {
            let (my_prec, my_assoc) = (Prec::PI, Some(Assoc::Right));
            let param_ty = print_expr(*param_ty, my_prec, my_assoc, false);
            let ret_ty = print_expr(*ret_ty, my_prec, my_assoc, true);

            let s = format!("{} -> {}", param_ty, ret_ty);

            maybe_parens(s, my_prec, prec, assoc, is_right_child)
        }
    }
}
