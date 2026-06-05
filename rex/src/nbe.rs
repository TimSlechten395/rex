use std::rc::Rc;

use crate::data::expr::{Expr, ExprF, GExpr};

#[derive(Clone)]
pub enum Value {
    Type,
    Pi(String, Rc<Value>, Closure),
    Lam(String, Rc<Value>, Closure),
    Neutral(Rc<Neutral>),
}

#[derive(Clone)]
pub enum Neutral {
    Var(usize),
    App(Rc<Neutral>, Rc<Value>),
}

pub type Env = Vec<Rc<Value>>;

fn lookup(env: &Env, idx: usize) -> Rc<Value> {
    env[env.len() - 1 - idx].clone()
}

#[derive(Clone)]
pub struct Closure {
    pub env: Env,
    pub body: Rc<Expr>,
}

impl Closure {
    fn apply(&self, arg: Rc<Value>) -> Rc<Value> {
        let mut env = self.env.clone();
        env.push(arg);
        eval(&env, &self.body)
    }
}

fn eval(env: &Env, tm: &Expr) -> Rc<Value> {
    match &tm.0 {
        ExprF::Var { idx: i } => lookup(env, *i),
        ExprF::Type => Rc::new(Value::Type),
        ExprF::Lambda {
            name,
            param_ty,
            body,
        } => Rc::new(Value::Lam(
            name.clone(),
            eval(env, &*param_ty),
            Closure {
                env: env.clone(),
                body: Rc::from(body.clone()),
            },
        )),

        ExprF::Pi {
            name,
            param_ty,
            ret_ty,
        } => Rc::new(Value::Pi(
            name.clone(),
            eval(env, &*param_ty),
            Closure {
                env: env.clone(),
                body: Rc::from(ret_ty.clone()),
            },
        )),
        ExprF::App { func, arg } => {
            let vf = eval(env, &*func);
            let va = eval(env, &*arg);
            vapp(vf, va)
        }
        _ => Rc::new(Value::Neutral(Rc::new(Neutral::Var(usize::MAX)))),
    }
}

fn vapp(fun: Rc<Value>, arg: Rc<Value>) -> Rc<Value> {
    match fun.as_ref() {
        Value::Lam(name, ty, clo) => clo.apply(arg),
        Value::Neutral(n) => Rc::new(Value::Neutral(Rc::new(Neutral::App(n.clone(), arg)))),
        _ => panic!("not a function"),
    }
}

fn quote(level: usize, v: Rc<Value>) -> Expr {
    let expr = match v.as_ref() {
        Value::Type => ExprF::Type,
        Value::Lam(name, ty, cl) => {
            let var = Rc::new(Value::Neutral(Rc::new(Neutral::Var(level))));
            let body_val = cl.apply(var);

            let param_ty = Box::new(quote(level, ty.clone()));

            ExprF::Lambda {
                name: name.clone(),
                param_ty,
                body: Box::new(quote(level + 1, body_val)),
            }
        }
        Value::Pi(name, a, cl) => {
            let param_ty = Box::new(quote(level, a.clone()));
            let var = Rc::new(Value::Neutral(Rc::new(Neutral::Var(level))));
            let ret_ty = Box::new(quote(level + 1, cl.apply(var)));

            ExprF::Pi {
                name: name.clone(),
                param_ty,
                ret_ty,
            }
        }
        Value::Neutral(n) => {
            return quote_neutral(level, n.clone());
        }
    };
    GExpr(expr)
}

fn quote_neutral(level: usize, n: Rc<Neutral>) -> Expr {
    let expr = match n.as_ref() {
        Neutral::Var(lvl) => {
            // convert level → de Bruijn index
            let idx = level - lvl - 1;
            ExprF::Var { idx }
        }

        Neutral::App(f, x) => ExprF::App {
            func: Box::new(quote_neutral(level, f.clone())),
            arg: Box::new(quote(level, x.clone())),
        },
    };
    GExpr(expr)
}
