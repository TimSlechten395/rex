use core::fmt;
use std::collections::HashMap;
use std::fmt::{Debug, write};
use std::fmt::{Display, Formatter};
use std::rc::Rc;

use anyhow::anyhow;
use anyhow::bail;
use either::Either::{self, Left, Right};
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
    Fix,
    TypeHole,
}

pub fn to_named(expr: &Expr) -> NamedExpr {
    let expr = match expr.clone().0 {
        ExprF::Var { idx } => ExprF::Var {
            idx: VarKind::Idx(idx),
        },
        ExprF::App { func, arg } => ExprF::App {
            func: Rc::new(to_named(&func)),
            arg: Rc::new(to_named(&arg)),
        },
        ExprF::Lambda {
            name,
            param_ty,
            body,
        } => ExprF::Lambda {
            name: VarKind::Idx(name),
            param_ty: Rc::new(to_named(&param_ty)),
            body: Rc::new(to_named(&body)),
        },
        ExprF::Pi {
            name,
            param_ty,
            ret_ty,
        } => ExprF::Pi {
            name: VarKind::Idx(name),
            param_ty: Rc::new(to_named(&param_ty)),
            ret_ty: Rc::new(to_named(&ret_ty)),
        },
        ExprF::Type => ExprF::Type,
        ExprF::Builtin(builtin) => ExprF::Builtin(builtin),
        ExprF::Err(expr_error, items) => ExprF::Err(
            expr_error,
            items.into_iter().map(|x| Rc::new(to_named(&x))).collect(),
        ),
    };
    GExpr(expr)
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
pub struct GExpr<A, B, E>(pub ExprF<Rc<GExpr<A, B, E>>, A, B, E>);

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

// impl<A: Debug + Clone, B: Debug + Clone, E: Debug + Clone> Traverse for GExpr<A, B, E> {
//     type Span = Vec<usize>;
//     fn traverse(self, path: Vec<usize>) -> anyhow::Result<Box<Self>> {
//         let mut path = path.into_iter();
//         let current = path.next();
//         match current {
//             Some(cur) => match self.0.clone() {
//                 ExprF::Builtin(..) => {
//                     bail!("invalid path in {:?}", cur)
//                 }
//                 ExprF::Var { .. } => {
//                     bail!("invalid path in {:?}", cur)
//                 }
//                 ExprF::App { func, arg } => match cur {
//                     0 => func.traverse(path.collect()),
//                     1 => arg.traverse(path.collect()),
//                     n => bail!("invalid path in {:?}", n),
//                 },
//                 ExprF::Lambda { param_ty, body, .. } => match cur {
//                     0 => param_ty.traverse(path.collect()),
//                     1 => body.traverse(path.collect()),
//                     n => bail!("invalid path in {:?}", n),
//                 },
//                 ExprF::Pi {
//                     param_ty, ret_ty, ..
//                 } => match cur {
//                     0 => param_ty.traverse(path.collect()),
//                     1 => ret_ty.traverse(path.collect()),
//                     _ => bail!("invalid path in {:?}", cur),
//                 },
//                 ExprF::Err(_, e) => e
//                     .get(cur)
//                     .ok_or_else(|| anyhow!("invalid path in {:?}", cur))
//                     .and_then(|x| x.clone().traverse(path.collect())),
//                 ExprF::Type => bail!("invalid path in {:?}", cur),
//             },
//
//             None => Ok(Box::new(self)),
//         }
//     }
// }

pub type Spanned<T> = (T, Vec<usize>);

#[derive(Debug, Clone)]
pub struct SpannedGExpr<A, B, E>(pub Spanned<ExprF<Box<SpannedGExpr<A, B, E>>, A, Spanned<B>, E>>);

pub type SpannedExpr = SpannedGExpr<usize, String, Vec<usize>>;
pub type SpannedNamedExpr =
    SpannedGExpr<VarKind<String, usize>, VarKind<String, String>, Vec<usize>>;

#[derive(Debug, Clone)]
pub struct GDef<A, B, E> {
    pub name: Spanned<String>,
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
            if loc.clone() == expr.name.1 {
                return vec![vec![i, 0]];
            }
            let mut ty = expr
                .ty
                .map(|ty| search(ty, loc.clone(), vec![i, 1]))
                .unwrap_or(vec![]);
            let val = search(expr.val, loc.clone(), vec![i, 2]);
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
                } => {
                    let mut ret = search(*one, loc.clone(), push_new(cur_loc.clone(), 0));
                    ret.extend(search(*two, loc.clone(), push_new(cur_loc.clone(), 1)));
                    ret
                }
                Lambda {
                    name: one,
                    param_ty: two,
                    body: three,
                }
                | Pi {
                    name: one,
                    param_ty: two,
                    ret_ty: three,
                } => {
                    let mut ret = search(*two, loc.clone(), push_new(cur_loc.clone(), 1));
                    ret.extend(search(*three, loc.clone(), push_new(cur_loc.clone(), 2)));
                    if loc == one.1 {
                        ret.push(push_new(cur_loc.clone(), 0));
                    }
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
            // TODO: for now only exact matches count
            // if ret.is_empty() {
            //     ret.push(cur_loc);
            // }
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
        let expr = match self.0.0 {
            ExprF::Var { idx } => ExprF::Var { idx },
            ExprF::App { func, arg } => ExprF::App { func, arg },
            ExprF::Lambda {
                name,
                param_ty,
                body,
            } => ExprF::Lambda {
                name: name.0,
                param_ty,
                body,
            },
            ExprF::Pi {
                name,
                param_ty,
                ret_ty,
            } => ExprF::Pi {
                name: name.0,
                param_ty,
                ret_ty,
            },
            ExprF::Type => ExprF::Type,
            ExprF::Builtin(builtin) => ExprF::Builtin(builtin),
            ExprF::Err(expr_error, items) => ExprF::Err(expr_error, items),
        };
        let expr = expr.fmap(|e| Rc::new(e.remove_span()));
        GExpr(expr)
    }
}

pub fn traverse_defs(
    expr: NamedDefs,
    path: Vec<usize>,
) -> anyhow::Result<either::Either<SpannedNamedExpr, Spanned<String>>> {
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
    let ret = match next {
        0 => Either::Right(expr.name.clone()),
        1 => expr
            .ty
            .clone()
            .ok_or(anyhow!("no type for this annotation"))?
            .traverse(remaining)?,

        2 => expr.val.clone().traverse(remaining)?,
        _ => bail!("pointed to def, but def was not available"),
    };
    Ok(ret)
}

impl<A: Debug + Clone, B: Debug + Clone + ToString, E: Debug + Clone> SpannedGExpr<A, B, E> {
    fn traverse(self, path: Vec<usize>) -> anyhow::Result<Either<Self, Spanned<String>>> {
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
                ExprF::Lambda {
                    name,
                    param_ty,
                    body,
                } => match cur {
                    0 => {
                        let name = (name.0.to_string(), name.1);
                        Ok(Right(name))
                    }
                    1 => param_ty.traverse(path.collect()),
                    2 => body.traverse(path.collect()),
                    _ => Err(err),
                },
                ExprF::Pi {
                    name,
                    param_ty,
                    ret_ty,
                } => match cur {
                    0 => {
                        let name = (name.0.to_string(), name.1);
                        Ok(Right(name))
                    }
                    1 => param_ty.traverse(path.collect()),
                    2 => ret_ty.traverse(path.collect()),
                    _ => Err(err),
                },
                ExprF::Err(_, v) => v
                    .get(cur)
                    .ok_or(err)
                    .and_then(|x| x.clone().traverse(path.collect())),
            },

            None => Ok(Left(self)),
        }
    }
}
