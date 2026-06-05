use core::fmt;
use std::collections::HashMap;
use std::fmt::{Debug, write};
use std::fmt::{Display, Formatter};

use anyhow::anyhow;
use anyhow::bail;
use functor_derive::Functor;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::Traverse;
use crate::helper::push_new;
use crate::pipeline::desugar::iter_with_loc;

// maybe split up T in value and type, most of the time they need to be the same but not always
//for examlpe, you might want to have an Option for the type but not for for the Value

#[derive(Debug, Clone, PartialEq, Eq, Hash, Functor, Serialize, Deserialize)]
pub enum ExprF<T, A, B, E> {
    Var { idx: A },
    App { func: T, arg: T },
    Lambda { name: B, param_ty: T, body: T },
    Pi { name: B, param_ty: T, ret_ty: T },
    Type,
    Builtin(Builtin),
    Err(ExprError<E>, Vec<T>),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Builtin {
    String(String),
    StringTy,
    // Struct {
    //     map: HashMap<String, Box<ExprF<T, A, B>>>,
    // },
    // Accessor,
    // {
    //     map: HashMap<String, Box<ExprF<T, A, B>>>,
    // },
    Num(usize),
    NumTy,
    Bool(bool),
    BoolTy,
    StringCmp,
    NumCmp,
    BoolCmp,
}

// Experimental: allow annotations everywhere and use them to derive missing types
#[derive(Debug, Clone, PartialEq, Eq, Hash, Functor)]
pub enum AnnExprF<T, A, B, E> {
    Expr(ExprF<Option<T>, A, B, E>),
    Ann(T, T),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Functor)]
pub struct AnnExpr<A, B, E>(AnnExprF<Box<AnnExpr<A, B, E>>, A, B, E>);

// this will require higher level unification but for now we will handle simple type annotations
fn convert<A: Clone, B: Clone, E: Clone>(ann_expr: AnnExpr<A, B, E>) -> Option<GExpr<A, B, E>> {
    match ann_expr.0 {
        AnnExprF::Expr(expr) => match expr {
            ExprF::Var { idx } => Some(GExpr(ExprF::Var { idx: idx })),
            ExprF::App { func, arg } => {
                let func_conv = convert(*func?)?;
                let arg_conv = convert(*arg?)?;
                Some(GExpr(ExprF::App {
                    func: Box::new(func_conv),
                    arg: Box::new(arg_conv),
                }))
            }
            ExprF::Lambda {
                name,
                param_ty,
                body,
            } => {
                let param_ty_conv = convert(*param_ty?)?;
                let body_conv = convert(*body?)?;

                Some(GExpr(ExprF::Lambda {
                    name,
                    param_ty: Box::new(param_ty_conv),
                    body: Box::new(body_conv),
                }))
            }
            ExprF::Pi {
                name,
                param_ty,
                ret_ty,
            } => {
                let param_ty_conv = convert(*param_ty?)?;
                let ret_ty = convert(*ret_ty?)?;

                Some(GExpr(ExprF::Pi {
                    name,
                    param_ty: Box::new(param_ty_conv),
                    ret_ty: Box::new(ret_ty),
                }))
            }

            ExprF::Type => todo!(),
            ExprF::Builtin(_) => todo!(),
            ExprF::Err(..) => todo!(),
        },
        AnnExprF::Ann(expr, ty) => todo!(),
    }
}

// TODO: remove parens based on prec and assoc
impl Display for NamedExpr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match &self.0 {
            ExprF::Builtin(s) => write!(f, "\"{s:?}\""),
            ExprF::Var { idx } => write!(f, "{idx}"),
            ExprF::Type => write!(f, "Type"),

            ExprF::App { func, arg } => write!(f, "({} {})", func, arg),

            ExprF::Lambda {
                name,
                param_ty,
                body,
            } => match name {
                VarKind::Named(s) => {
                    write!(f, "(({}: {}) => {})", s, param_ty, body)
                }
                VarKind::Idx(_) => {
                    write!(f, "({} => {})", param_ty, body)
                }
            },

            ExprF::Pi {
                name,
                param_ty,
                ret_ty,
            } => match name {
                VarKind::Named(s) => {
                    write!(f, "(({}: {}) -> {})", s, param_ty, ret_ty)
                }
                VarKind::Idx(_) => {
                    write!(f, "({} -> {})", param_ty, ret_ty)
                }
            },
            ExprF::Err(..) => write!(f, "err"),
        }
    }
}

impl<T, A, B, E> ExprF<T, A, B, E> {
    pub fn fold<U>(self, init: U, f: impl Fn(U, T) -> U + Clone) -> U {
        match self {
            ExprF::Var { .. } => init,
            ExprF::App { func, arg } => f(f(init, func), arg),
            ExprF::Lambda { param_ty, body, .. } => f(f(init, param_ty), body),
            ExprF::Pi {
                param_ty, ret_ty, ..
            } => f(f(init, param_ty), ret_ty),
            ExprF::Type | ExprF::Builtin(_) => init,
            ExprF::Err(_, e) => e.into_iter().fold(init, f),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct GExpr<A, B, E>(pub ExprF<Box<GExpr<A, B, E>>, A, B, E>);

// The string here is for the name hint
pub type Expr = GExpr<usize, String, Vec<usize>>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum VarKind<A, B> {
    Named(A),
    Idx(B),
}

impl<A: Display, B: Display> Display for VarKind<A, B> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            VarKind::Named(name) => write!(f, "{name}"),
            VarKind::Idx(idx) => write!(f, "${idx}"),
        }
    }
}

pub type NamedExpr = GExpr<VarKind<String, usize>, VarKind<String, String>, Vec<usize>>;

impl<A: Debug + Clone, B: Debug + Clone, E: Debug + Clone> Traverse for GExpr<A, B, E> {
    type Span = Vec<usize>;
    fn traverse(self, path: Vec<usize>) -> anyhow::Result<Box<Self>> {
        let mut path = path.into_iter();
        let current = path.next();
        match current {
            Some(cur) => match self.0.clone() {
                ExprF::Builtin(..) => {
                    bail!("invalid path in {:?}", cur)
                }
                ExprF::Var { .. } => {
                    bail!("invalid path in {:?}", cur)
                }
                ExprF::App { func, arg } => match cur {
                    0 => func.traverse(path.collect()),
                    1 => arg.traverse(path.collect()),
                    n => bail!("invalid path in {:?}", n),
                },
                ExprF::Lambda { param_ty, body, .. } => match cur {
                    0 => param_ty.traverse(path.collect()),
                    1 => body.traverse(path.collect()),
                    n => bail!("invalid path in {:?}", n),
                },
                ExprF::Pi {
                    param_ty, ret_ty, ..
                } => match cur {
                    0 => param_ty.traverse(path.collect()),
                    1 => ret_ty.traverse(path.collect()),
                    _ => bail!("invalid path in {:?}", cur),
                },
                ExprF::Err(_, e) => e
                    .get(cur)
                    .ok_or_else(|| anyhow!("invalid path in {:?}", cur))
                    .and_then(|x| x.clone().traverse(path.collect())),
                ExprF::Type => bail!("invalid path in {:?}", cur),
            },

            None => Ok(Box::new(self)),
        }
    }
}

pub type Spanned<T> = (T, Vec<usize>);

#[derive(Debug, Clone)]
pub struct SpannedGExpr<A, B, E>(pub Spanned<ExprF<Box<SpannedGExpr<A, B, E>>, A, B, E>>);

pub type SpannedExpr = SpannedGExpr<usize, String, Vec<usize>>;
pub type SpannedNamedExpr =
    SpannedGExpr<VarKind<String, usize>, VarKind<String, String>, Vec<usize>>;

#[derive(Debug, Clone)]
pub struct GDef<A, B, E> {
    pub name: String,
    pub ty: Option<SpannedGExpr<A, B, E>>,
    pub val: SpannedGExpr<A, B, E>,
}

pub type Def = GDef<usize, String, Vec<usize>>;
pub type NamedDef = GDef<VarKind<String, usize>, VarKind<String, String>, Vec<usize>>;

#[derive(Debug, Clone)]
pub struct GDefs<A, B, E>(pub Vec<GDef<A, B, E>>);

pub type Defs = GDefs<usize, String, Vec<usize>>;
pub type NamedDefs = GDefs<VarKind<String, usize>, VarKind<String, String>, Vec<usize>>;

#[derive(Debug, Clone, Functor, Error, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ExprError<T> {
    #[error("missing binder: {0:?}")]
    MissingBinder(T),

    #[error("missing value: {0:?}")]
    MissingValue(T),

    #[error("missing type: {0:?}")]
    MissingType(T),

    #[error("invalid binder: {0:?}")]
    InvalidBinder(T),

    #[error("invalid binder name: {0:?}")]
    InvalidBinderName(T),

    #[error("invalid binder param: {0:?}")]
    InvalidBinderParam(T),

    #[error("arbitrary binders are not supported: {0:?}")]
    ArbitraryBinding(T),

    #[error("arbitrary type annotation is not supported: {0:?}")]
    ArbitraryAnn(T),

    #[error("failed to resolve variable {0:?} at {1:?} ")]
    ResolveFailed(String, (Vec<usize>, Vec<VarKind<String, String>>)),

    #[error("Unknown")]
    Unknown,
}

// impl<A, B> Compile for SpannedExpr<A, B>

// in rare cases a token might be represented in multiple parts of the ast
pub fn search_defs(expr: NamedDefs, loc: Vec<usize>) -> Vec<Vec<usize>> {
    expr.0
        .clone()
        .into_iter()
        .enumerate()
        .map(|(i, expr)| {
            let mut ty = expr
                .ty
                .map(|ty| search(ty, loc.clone(), vec![i, 0]))
                .unwrap_or(vec![]);
            let val = search(expr.val, loc.clone(), vec![i, 1]);
            ty.extend(val);
            ty
        })
        .find(|x| !x.is_empty())
        .unwrap_or(vec![])
}
//
pub fn search<A: Clone + Debug, B: Clone + Debug, E: Clone + Debug>(
    expr: SpannedGExpr<A, B, E>,
    loc: Vec<usize>,
    cur_loc: Vec<usize>,
) -> Vec<Vec<usize>> {
    use ExprF::*;
    let range = expr.0.1;
    if loc.starts_with(&range) {
        if loc.len() == range.len() {
            vec![cur_loc]
        } else {
            let mut ret = match expr.0.0.clone() {
                Var { .. } | Type | Builtin(..) => vec![],
                App {
                    func: one,
                    arg: two,
                }
                | Lambda {
                    name: _,
                    param_ty: one,
                    body: two,
                }
                | Pi {
                    name: _,
                    param_ty: one,
                    ret_ty: two,
                } => {
                    let mut ret = search(*one, loc.clone(), push_new(cur_loc.clone(), 0));
                    ret.extend(search(*two, loc.clone(), push_new(cur_loc.clone(), 1)));
                    ret
                }
                Err(_, items) => iter_with_loc(items, cur_loc.clone())
                    .map(|x| search(*x.0, loc.clone(), x.1))
                    .reduce(|mut acc, e| {
                        acc.extend(e);
                        acc
                    })
                    .unwrap_or(Vec::new()),
            };
            if ret.is_empty() {
                ret.push(cur_loc);
            }
            ret
        }
    } else {
        vec![]
    }
}
//
impl<A, B, E> SpannedGExpr<A, B, E> {
    // pub fn search(self, loc: Vec<usize>) -> Vec<Vec<usize>> {
    //     let mut loc2 = loc.clone().into_iter();
    //
    //     for p in self.0.1 {
    //         if p != loc2.next()? {
    //             return None;
    //         }
    //     }
    //
    //     match self.0.0 {
    //         ExprF::Var { idx } => {}
    //         }
    //         ExprF::App { func, arg } => self.search(func, lo) && self.search(),
    //         ExprF::Lambda {
    //             name,
    //             param_ty,
    //             body,
    //         } => todo!(),
    //         ExprF::Pi {
    //             name,
    //             param_ty,
    //             ret_ty,
    //         } => todo!(),
    //         ExprF::Type => todo!(),
    //         ExprF::Builtin(builtin) => todo!(),
    //     }
    // }

    pub fn remove_span(self) -> GExpr<A, B, E> {
        let expr = self.0.0.fmap(|e| Box::new(e.remove_span()));
        GExpr(expr)
    }
}

pub fn traverse_defs(expr: NamedDefs, path: Vec<usize>) -> anyhow::Result<Box<SpannedNamedExpr>> {
    let mut iter = path.into_iter();
    let Some(next) = iter.next() else {
        bail!("path was completely empty")
    };
    let Some(expr) = expr.0.get(next) else {
        bail!("index out of bounds: len: {} got: {}", expr.0.len(), next)
    };

    let Some(next) = iter.next() else {
        bail!("did not specify which part of def")
    };
    let remaining = iter.collect();
    match next {
        0 => expr
            .ty
            .clone()
            .ok_or(anyhow!("no type for this annotation"))?
            .traverse(remaining),
        1 => expr.val.clone().traverse(remaining),
        _ => bail!("pointed to def, but def was not available"),
    }
}

impl<A: Debug + Clone, B: Debug + Clone, E: Debug + Clone> Traverse for SpannedGExpr<A, B, E> {
    type Span = Vec<usize>;
    fn traverse(self, path: Vec<usize>) -> anyhow::Result<Box<Self>> {
        let mut path = path.into_iter();

        let s = format!("{:?}", &self.0.0);
        let dbg = match s.char_indices().nth(50) {
            Some((i, _)) => format!("{}...", &s[..i]),
            None => s,
        };

        let err = anyhow!(format!("invalid path in {}, path: {:?}", dbg, path));
        let current = path.next();

        match current {
            Some(cur) => match self.0.0.clone() {
                ExprF::Var { .. } | ExprF::Type | ExprF::Builtin(_) => Err(err),
                ExprF::App { func, arg } => match cur {
                    0 => func.traverse(path.collect()),
                    1 => arg.traverse(path.collect()),
                    _ => Err(err),
                },
                ExprF::Lambda { param_ty, body, .. } => match cur {
                    0 => param_ty.traverse(path.collect()),
                    1 => body.traverse(path.collect()),
                    _ => Err(err),
                },
                ExprF::Pi {
                    param_ty, ret_ty, ..
                } => match cur {
                    0 => param_ty.traverse(path.collect()),
                    1 => ret_ty.traverse(path.collect()),
                    _ => Err(err),
                },
                ExprF::Err(_, v) => v
                    .get(cur)
                    .ok_or(err)
                    .and_then(|x| x.clone().traverse(path.collect())),
            },

            None => Ok(Box::new(self)),
        }
    }
}
