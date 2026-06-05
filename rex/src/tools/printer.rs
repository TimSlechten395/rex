use std::fmt::format;

use crate::{
    data::expr::{Expr, ExprF, NamedExpr, VarKind},
    helper::push_new,
};

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

pub fn print_named_expr(code: NamedExpr) -> String {
    fn go(code: NamedExpr, prec: Prec, assoc: Option<Assoc>, is_right_child: bool) -> String {
        use ExprF::*;

        match code.0 {
            Err(..) => "err".to_string(),
            Builtin(s) => match s {
                crate::data::expr::Builtin::String(s) => format!("\"{}\"", s),
                s => "__something builtin__".to_string(),
            },
            Var { idx } => match idx {
                VarKind::Named(s) => s,
                VarKind::Idx(i) => i.to_string(),
            },

            Type => "Type".to_string(),

            App { func, arg } => {
                let (my_prec, my_assoc) = (Prec::APP, Some(Assoc::Left));
                let s = format!(
                    "{} {}",
                    go(*func, my_prec, my_assoc, false),
                    go(*arg, my_prec, my_assoc, true)
                );
                maybe_parens(s, my_prec, prec, assoc, is_right_child)
            }

            Lambda {
                name,
                param_ty,
                body,
            } => {
                let (my_prec, my_assoc) = (Prec::LAMBDA, Some(Assoc::Right));
                let body_print = go(*body, my_prec, my_assoc, true);

                let s = match name {
                    VarKind::Named(name) => {
                        let param_ty_print = go(*param_ty, Prec::LOWEST, my_assoc, false);
                        format!("({}: {}) => {}", name, param_ty_print, body_print)
                    }
                    VarKind::Idx(_) => {
                        let param_ty_print = go(*param_ty, my_prec, my_assoc, false);
                        format!("{} => {}", param_ty_print, body_print)
                    }
                };

                maybe_parens(s, my_prec, prec, assoc, is_right_child)
            }

            Pi {
                name,
                param_ty,
                ret_ty,
            } => {
                let (my_prec, my_assoc) = (Prec::PI, Some(Assoc::Right));
                let ret_ty_print = go(*ret_ty, my_prec, my_assoc, true);

                let s = match name {
                    VarKind::Named(name) => {
                        let param_ty_print = go(*param_ty, Prec::LOWEST, my_assoc, false);
                        format!("({}: {}) -> {}", name, param_ty_print, ret_ty_print)
                    }
                    VarKind::Idx(_) => {
                        let param_ty_print = go(*param_ty, my_prec, my_assoc, false);
                        format!("{} -> {}", param_ty_print, ret_ty_print)
                    }
                };

                maybe_parens(s, my_prec, prec, assoc, is_right_child)
            }
        }
    }
    go(code, Prec::LOWEST, None, false)
}

fn is_church(expr: &Expr) -> Option<usize> {
    let body = match &expr.0 {
        ExprF::Lambda {
            param_ty, body: b1, ..
        } => {
            if ExprF::Type == param_ty.0 {
                match &b1.0 {
                    // second param type must be A → A, i.e. Pi(_, Var(0), Var(0))
                    ExprF::Lambda {
                        param_ty, body: b2, ..
                    } => {
                        match &param_ty.0 {
                            ExprF::Pi {
                                param_ty: pty,
                                ret_ty,
                                ..
                            } => match (&pty.0, &ret_ty.0) {
                                (ExprF::Var { idx: 0 }, ExprF::Var { idx: 1 }) => {}
                                _ => return None,
                            },
                            _ => return None,
                        }
                        match &b2.0 {
                            ExprF::Lambda {
                                body: b3, param_ty, ..
                            } => {
                                match &param_ty.0 {
                                    ExprF::Var { idx: 1 } => {}
                                    _ => return None,
                                }
                                b3
                            }
                            _ => return None,
                        }
                    }
                    _ => return None,
                }
            } else {
                return None;
            }
        }
        _ => return None,
    };
    count_apps(body)
}

fn count_apps(expr: &Expr) -> Option<usize> {
    match &expr.0 {
        ExprF::Var { idx: 0 } => Some(0), // x
        ExprF::App { func, arg } => match &func.0 {
            ExprF::Var { idx: 1 } => Some(1 + count_apps(arg)?), // f applied n times
            _ => None,
        },
        _ => None,
    }
}

pub fn print_expr(code: Expr) -> String {
    pub fn go(
        code: Expr,
        prec: Prec,
        assoc: Option<Assoc>,
        is_right_child: bool,
        ctx: &mut Vec<String>,
    ) -> String {
        use ExprF::*;

        if let Some(n) = is_church(&code) {
            return n.to_string();
        }

        match code.0 {
            Err(..) => "err".to_string(),
            Builtin(s) => match s {
                crate::data::expr::Builtin::String(s) => format!("\"{}\"", s),
                s => "__something builtin__".to_string(),
            },
            Var { idx } => {
                let name = ctx.iter().rev().nth(idx);
                if let Some(name) = name
                    && !name.is_empty()
                {
                    name.clone()
                } else {
                    "#".to_string() + &idx.to_string()
                }
            }

            Type => "Type".to_string(),

            App { func, arg } => {
                let (my_prec, my_assoc) = (Prec::APP, Some(Assoc::Left));
                let s = format!(
                    "{} {}",
                    go(*func, my_prec, my_assoc, false, ctx),
                    go(*arg, my_prec, my_assoc, true, ctx)
                );
                maybe_parens(s, my_prec, prec, assoc, is_right_child)
            }

            Lambda {
                name,
                param_ty,
                body,
            } => {
                let (my_prec, my_assoc) = (Prec::LAMBDA, Some(Assoc::Right));
                let param_ty = go(*param_ty, my_prec, my_assoc, false, ctx);

                ctx.push(name.clone());
                let body = go(*body, my_prec, my_assoc, true, ctx);
                ctx.pop();

                let s = if !name.is_empty() {
                    format!("({}: {}) => {}", name, param_ty, body)
                } else {
                    format!("{} => {}", param_ty, body)
                };

                maybe_parens(s, my_prec, prec, assoc, is_right_child)
            }

            Pi {
                name,
                param_ty,
                ret_ty,
            } => {
                let (my_prec, my_assoc) = (Prec::PI, Some(Assoc::Right));
                let param_ty = go(*param_ty, my_prec, my_assoc, false, ctx);

                ctx.push(name.clone());
                let ret_ty = go(*ret_ty, my_prec, my_assoc, true, ctx);
                ctx.pop();

                let s = if !name.is_empty() {
                    format!("({}: {}) -> {}", name, param_ty, ret_ty)
                } else {
                    format!("{} -> {}", param_ty, ret_ty)
                };

                maybe_parens(s, my_prec, prec, assoc, is_right_child)
            }
        }
    }

    go(code, Prec::LOWEST, None, false, &mut Vec::new())
}
