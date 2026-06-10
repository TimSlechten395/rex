use std::{collections::HashMap, error::Error, rc::Rc, sync::Arc};

use anyhow::anyhow;

use crate::{
    bootstrap::compile_min_version,
    data::expr::{Builtin, Expr, ExprF, GExpr},
    tools::printer::print_expr,
    r#type::infer_type,
};

// if you beta reduce under a lambda the free variables need to be shifted accordingly
pub fn shift(expr: &Expr, delta: isize, cutoff: usize) -> Expr {
    let expr = match &expr.0 {
        ExprF::Var { idx } => {
            if *idx >= cutoff {
                // k + d
                let k_i = *idx as isize + delta;
                assert!(k_i >= 0, "shift resulted in negative index {k_i:?}");
                ExprF::Var { idx: k_i as usize }
            } else {
                ExprF::Var { idx: *idx }
            }
        }
        ExprF::Lambda {
            param_ty,
            body,
            name,
        } => {
            let param_ty = Rc::new(shift(param_ty, delta, cutoff));
            let body = Rc::new(shift(body, delta, cutoff + 1));
            ExprF::Lambda {
                name: name.clone(),
                param_ty,
                body,
            }
        }
        ExprF::Pi {
            param_ty,
            ret_ty,
            name,
        } => {
            let param_ty = Rc::new(shift(param_ty, delta, cutoff));
            let ret_ty = Rc::new(shift(ret_ty, delta, cutoff + 1));
            ExprF::Pi {
                name: name.clone(),
                param_ty,
                ret_ty,
            }
        }
        ExprF::App { func, arg } => {
            let func = Rc::new(shift(func, delta, cutoff));
            let arg = Rc::new(shift(arg, delta, cutoff));
            ExprF::App { func, arg }
        }
        other => other.clone(),
    };
    GExpr(expr)
}

// do we need the index here?
pub fn subst(index: usize, body: &Expr, arg: &Expr) -> Expr {
    let expr = match &body.0 {
        ExprF::Var { idx } => {
            if *idx == index {
                arg.0.clone()
            } else {
                body.0.clone()
            }
        }
        //
        ExprF::Lambda {
            param_ty,
            body,
            name,
        } => {
            // The only important part of this function
            let param_ty = Rc::new(subst(index, param_ty, arg));
            let arg_shifted = shift(&arg, 1, 0);
            let body = Rc::new(subst(index + 1, body, &arg_shifted));
            ExprF::Lambda {
                name: name.clone(),
                param_ty,
                body,
            }
            // This might be a problem
        }
        ExprF::App {
            func: app_func,
            arg: app_arg,
        } => {
            let app_func = Rc::new(subst(index, app_func, arg));
            let app_arg = Rc::new(subst(index, app_arg, arg));
            ExprF::App {
                func: app_func,
                arg: app_arg,
            }
        }
        ExprF::Pi {
            param_ty,
            ret_ty,
            name,
        } => {
            let param_ty = Rc::new(subst(index, param_ty, arg));

            let arg_shifted = shift(&arg, 1, 0);
            let ret_ty = Rc::new(subst(index + 1, ret_ty, &arg_shifted));

            ExprF::Pi {
                name: name.clone(),
                param_ty,
                ret_ty,
            }
        }
        func => func.clone(),
    };
    GExpr(expr)
}

pub fn beta_reduce(body: &Expr, arg: &Expr) -> Expr {
    let arg_shifted = shift(arg, 1, 0);
    let substed = subst(0, body, &arg_shifted);
    shift(&substed, -1, 0)
}

pub fn weak_head_normal_form(expr: &Expr) -> Expr {
    match &expr.0 {
        ExprF::App { func, arg } => {
            let func_eval = weak_head_normal_form(func);

            match &func_eval.0 {
                ExprF::Lambda { body, .. } => {
                    let result = beta_reduce(body, arg);
                    weak_head_normal_form(&result)
                }
                // handles fixpoint application fix A f => (f A) (fix A f)
                ExprF::App {
                    func: fix,
                    arg: _fix_ty,
                } => match &fix.0 {
                    ExprF::Builtin(Builtin::Fix) => {
                        let result = GExpr(ExprF::App {
                            func: arg.clone(),
                            arg: Rc::new(GExpr(ExprF::App {
                                func: Rc::new(func_eval),
                                arg: arg.clone(),
                            })),
                        });

                        weak_head_normal_form(&result)
                    }
                    _ => GExpr(ExprF::App {
                        func: Rc::new(func_eval),
                        arg: arg.clone(),
                    }),
                },

                _ => GExpr(ExprF::App {
                    func: Rc::new(func_eval),
                    arg: arg.clone(),
                }),
            }
        }
        other => GExpr(other.clone()),
    }
}

pub fn head_normal_form(expr: &Expr) -> Expr {
    match &expr.0 {
        ExprF::Lambda {
            param_ty,
            body,
            name,
        } => {
            let param_ty = Rc::new(head_normal_form(param_ty));
            let body = Rc::new(head_normal_form(body));
            GExpr(ExprF::Lambda {
                name: name.clone(),
                param_ty,
                body,
            })
        }
        ExprF::Pi {
            param_ty,
            ret_ty,
            name,
        } => {
            let param_ty = Rc::new(head_normal_form(param_ty));
            let ret_ty = Rc::new(head_normal_form(ret_ty));
            GExpr(ExprF::Pi {
                name: name.clone(),
                param_ty,
                ret_ty,
            })
        }
        ExprF::App { func, arg } => {
            let func_eval = head_normal_form(func);

            match &func_eval.0 {
                ExprF::Lambda { body, .. } => {
                    let result = beta_reduce(body, arg);
                    head_normal_form(&result)
                }
                ExprF::App {
                    func: fix,
                    arg: fix_ty,
                } => match &fix.0 {
                    ExprF::Builtin(Builtin::Fix) => {
                        let result = GExpr(ExprF::App {
                            func: arg.clone(),
                            arg: Rc::new(GExpr(ExprF::App {
                                func: Rc::new(func_eval),
                                arg: arg.clone(),
                            })),
                        });
                        head_normal_form(&result)
                    }
                    _ => GExpr(ExprF::App {
                        func: Rc::new(func_eval),
                        arg: arg.clone(),
                    }),
                },
                _ => GExpr(ExprF::App {
                    func: Rc::new(func_eval),
                    arg: arg.clone(),
                }),
            }
        }
        _ => expr.clone(),
    }
}

pub fn normal_form(expr: &Expr) -> Expr {
    match &expr.0 {
        ExprF::Lambda {
            param_ty,
            body,
            name,
        } => {
            let param_ty = Rc::new(normal_form(param_ty));
            let body = Rc::new(normal_form(body));
            GExpr(ExprF::Lambda {
                name: name.clone(),
                param_ty,
                body,
            })
        }
        ExprF::Pi {
            param_ty,
            ret_ty,
            name,
        } => {
            let param_ty = Rc::new(normal_form(param_ty));
            let ret_ty = Rc::new(normal_form(ret_ty));
            GExpr(ExprF::Pi {
                name: name.clone(),
                param_ty,
                ret_ty,
            })
        }
        ExprF::App { func, arg } => {
            let func_eval = normal_form(func);

            match &func_eval.clone().0 {
                ExprF::Lambda { body, .. } => normal_form(&beta_reduce(body, arg)),
                ExprF::App {
                    func: fix,
                    arg: fix_ty,
                } => match fix.0 {
                    ExprF::Builtin(Builtin::Fix) => {
                        let result = GExpr(ExprF::App {
                            func: arg.clone(),
                            arg: Rc::new(GExpr(ExprF::App {
                                func: Rc::new(func_eval),
                                arg: arg.clone(),
                            })),
                        });
                        normal_form(&result)
                    }
                    _ => GExpr(ExprF::App {
                        func: Rc::new(func_eval),
                        arg: arg.clone(),
                    }),
                },
                _ => GExpr(ExprF::App {
                    func: Rc::new(func_eval),
                    arg: Rc::new(normal_form(arg)),
                }),
            }
        }
        _ => expr.clone(),
    }
}

pub fn optimize(expr: &Expr) -> Result<Arc<Expr>, Box<dyn Error>> {
    let ty = infer_type(&expr, vec![], &mut vec![], vec![])?;
    let defs = "def Nat: Type := (A: Type) -> (A -> A) -> A -> A
                def Bool: Type := (A: Type) -> A -> A -> A
                def add (n: Nat) (m: Nat): Nat := (A: Type) => (f: A -> A) => (x: A) => m A f (n A f x)";

    let defs: HashMap<_, _> = compile_min_version(defs)?.into_iter().collect();
    if ty == *defs.get("Nat").unwrap() {
        println!("got hello");
    }
    todo!()
}
