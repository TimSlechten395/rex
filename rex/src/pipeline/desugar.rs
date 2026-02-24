use num_bigint::BigUint;

use crate::{
    Compile,
    data::{
        ast::{Ast, FixAst, LitKind},
        expr::{
            ExprError, ExprF, GExpr, NamedExpr, Spanned, SpannedExpr, SpannedNamedExpr, VarKind,
        },
    },
    helper::push_new,
};

pub struct Desugar;

impl Compile for Desugar {
    type Input = FixAst;
    type Output = SpannedNamedExpr;
    type Error = ExprError<Vec<usize>>;

    fn run(input: Self::Input) -> Result<Self::Output, Self::Error> {
        desugar(input, Vec::new())
    }
}

// (R : Type) => (f : (x : A1) -> (y : A2 x) -> (z : A3 x y) -> R) => f x y z
// pub fn create_church_accessors(expr: NamedExpr, field_name: String) -> Option<NamedExpr> {
//     match expr.0 {
//         Expr::Pi {
//             name,
//             param_ty,
//             ret_ty,
//         } => {
//             let r_name = name;
//             match param_ty.0 {
//                 Expr::Type => match ret_ty.0 {
//                     Expr::Pi {
//                         name,
//                         param_ty,
//                         ret_ty,
//                     } => {
//                         let mut base = FixExpr(Expr::Var {
//                             idx: VarKind::Named(field_name),
//                         });
//
//                         let mut rest = param_ty;
//                         let f_name = name;
//
//                         while let Expr::Pi {
//                             name,
//                             param_ty,
//                             ret_ty,
//                         } = rest.0
//                         {
//                             base = FixExpr(Expr::Lambda {
//                                 name: name,
//                                 param_ty: param_ty,
//                                 body: Box::new(base),
//                             });
//                             rest = ret_ty;
//                         }
//                         Some(base)
//                     }
//                     _ => None,
//                 },
//                 _ => None,
//             }
//         }
//         _ => None,
//     }
// }
pub fn create_church_num(num: BigUint) -> NamedExpr {
    let mut body: NamedExpr = GExpr(ExprF::Var {
        idx: VarKind::Idx(0),
    });

    for _ in 0..num.to_u64_digits()[0] {
        body = GExpr(ExprF::App {
            func: Box::new(GExpr(ExprF::Var {
                idx: VarKind::Idx(1),
            })),
            arg: Box::new(body),
        });
    }

    GExpr(ExprF::Lambda {
        name: VarKind::Idx(()),
        param_ty: Box::new(GExpr(ExprF::Type)),
        body: Box::new(GExpr(ExprF::Lambda {
            name: VarKind::Idx(()),
            param_ty: Box::new(GExpr(ExprF::Pi {
                name: VarKind::Idx(()),
                param_ty: Box::new(GExpr(ExprF::Var {
                    idx: VarKind::Idx(0),
                })),
                ret_ty: Box::new(GExpr(ExprF::Var {
                    idx: VarKind::Idx(1),
                })),
            })),
            body: Box::new(GExpr(ExprF::Lambda {
                name: VarKind::Idx(()),
                param_ty: Box::new(GExpr(ExprF::Var {
                    idx: VarKind::Idx(1),
                })),
                body: Box::new(body),
            })),
        })),
    })
}

pub fn create_string(s: String) -> NamedExpr {
    let bit_ty = "(R: Type) -> R -> R -> R";
    let list_ty = "(A: Type) => (R: Type) -> (A -> R -> R) -> R -> R";

    let t = "(R: Type) => (t: R) => (f: R) => t";
    let f = "(R: Type) => (t: R) => (f: R) => f";

    let nil = "(A: Type) => (R: Type) => (f: A -> R -> R) -> (z: R) => z";

    // (A: Type) -> A -> list A -> list A
    let cons = "(A: Type) => (head: A) => (tail: list A) => (R: Type) => (f: A -> R -> R) => (z: R) => f head (tail R f z)";

    let bytes = s.into_bytes();

    let nat = "(R: Type) -> (R -> R) -> R -> R";
    let array = "(A: Type) => (n: Nat) => (R: Type) -> n Type ((X: Type) => (A -> X)) R -> R";

    todo!("implement string literals: {s}")
}

pub fn create_accessor(arity: usize, selected: usize) -> NamedExpr {
    assert!(arity > 0, "arity must be positive");
    assert!(selected <= arity, "selected out of range");

    // Build λx1. λx2. ... λxn. xi
    let mut body = GExpr(ExprF::Var {
        idx: VarKind::Idx(selected),
    });
    for _ in 0..arity {
        body = GExpr(ExprF::Lambda {
            name: VarKind::Idx(()),
            param_ty: Box::new(GExpr(ExprF::Type)),
            body: Box::new(body),
        });
    }

    body
}

pub type FullBinder = (Spanned<String>, Vec<Spanned<TypedBinding>>);

#[derive(Debug, Clone)]
pub struct TypedBinding {
    name: Option<Spanned<String>>,
    ty: Option<SpannedNamedExpr>,
    val: Option<SpannedNamedExpr>,
}

pub fn iter_with_loc<T: Clone>(
    items: Vec<T>,
    loc: Vec<usize>,
) -> impl DoubleEndedIterator<Item = (T, Vec<usize>)> + Clone {
    items
        .into_iter()
        .enumerate()
        .map(move |(new_loc, expr)| (expr, push_new(loc.clone(), new_loc)))
}

pub fn extract_binder(expr: FixAst, loc: Vec<usize>) -> Result<FullBinder, ExprError<Vec<usize>>> {
    use Ast::*;
    match expr.0 {
        Group(inner) => extract_binder(*inner, push_new(loc, 0)),
        Var(x) => Ok((((x, loc.clone())), Vec::new())),
        App(items) => {
            let mut items = iter_with_loc(items, loc.clone());

            let name = items.next().unwrap();

            let name_as_str = match name.0.0 {
                Var(name) => name,
                _ => {
                    return Err(ExprError::InvalidBinderName(name.1));
                }
            };

            let items = items.map(|(expr, loc)| {
                extract_binding(expr, loc.clone(), BindingPriority::NameTyVal)
                    .map(|x| (x, loc.clone()))
            });

            Ok((
                (name_as_str, name.1.clone()),
                items.collect::<Result<_, _>>()?,
            ))
        }
        _ => Err(ExprError::InvalidBinder(loc.clone())),
    }
}

pub enum BindingPriority {
    NameTyVal, // used for lambdas
    TyNameVal, // used for pis and sigmas
    ValNameTy, // used for tuples
}

pub fn fold_ty(
    binder: FullBinder,
    ty: Result<SpannedExpr<VarKind<String, usize>, VarKind<String, ()>>, ExprError<Vec<usize>>>,
) -> Result<SpannedExpr<VarKind<String, usize>, VarKind<String, ()>>, ExprError<Vec<usize>>> {
    binder
        .1
        .clone()
        .into_iter()
        .clone()
        .rev()
        .fold(ty, |acc, item| {
            Ok(SpannedExpr((
                ExprF::Pi {
                    name: item
                        .0
                        .name
                        .clone()
                        .map(|x| VarKind::Named(x.0))
                        .unwrap_or(VarKind::Idx(())),
                    param_ty: Box::new(
                        item.0
                            .ty
                            .clone()
                            .ok_or(ExprError::InvalidBinderParam(item.1.clone()))?,
                    ),
                    ret_ty: Box::new(acc?),
                },
                item.1,
            )))
        })
}

pub fn fold_val(
    binder: FullBinder,
    body: Result<SpannedExpr<VarKind<String, usize>, VarKind<String, ()>>, ExprError<Vec<usize>>>,
) -> Result<SpannedExpr<VarKind<String, usize>, VarKind<String, ()>>, ExprError<Vec<usize>>> {
    binder
        .1
        .clone()
        .into_iter()
        .clone()
        .rev()
        .fold(body, |acc, item| {
            Ok(SpannedExpr((
                ExprF::Lambda {
                    name: item
                        .0
                        .name
                        .clone()
                        .map(|x| VarKind::Named(x.0))
                        .unwrap_or(VarKind::Idx(())),
                    // This is a problem it needs to stay empty
                    param_ty: Box::new(
                        item.0
                            .ty
                            .clone()
                            .ok_or(ExprError::InvalidBinderParam(item.1.clone()))?,
                    ),
                    body: Box::new(acc?),
                },
                item.1,
            )))
        })
}

pub fn extract_binding(
    expr: FixAst,
    loc: Vec<usize>,
    prior: BindingPriority,
) -> Result<TypedBinding, ExprError<Vec<usize>>> {
    use Ast::*;
    let binding = match expr.0 {
        // first item is name second is val
        Group(inner) => extract_binding(*inner, push_new(loc.clone(), 0), prior)?,
        Binding(items) => {
            let mut items = iter_with_loc(items, loc.clone());

            let named = items.next().unwrap();

            let val = items.map(|(expr, loc)| desugar(expr, loc)).last().unwrap();

            match named.0.0 {
                Ann(items) => {
                    let mut items = iter_with_loc(items, named.1.clone());
                    let binder_expr = items.next().unwrap();

                    let ty = items.map(|(expr, i)| desugar(expr, i)).next().unwrap();

                    let binder = extract_binder(binder_expr.0.clone(), binder_expr.1.clone())?;

                    // converting 'succ (n : Nat) : Nat' into 'succ: (n: Nat) -> Nat'
                    let ty = fold_ty(binder.clone(), ty)?;
                    let val = fold_val(binder.clone(), val)?;

                    TypedBinding {
                        name: Some(binder.0),
                        ty: Some(ty),
                        val: Some(val),
                    }
                }
                _ => {
                    let binder = extract_binder(named.0.clone(), named.1.clone())?;

                    let val = fold_val(binder.clone(), val)?;

                    TypedBinding {
                        name: Some(binder.0),
                        ty: None,
                        val: Some(val),
                    }
                }
            }
        }
        Ann(items) => {
            let mut items = iter_with_loc(items, loc.clone());
            let binder_or_val = items.next().unwrap();

            let ty = items.map(|(expr, loc)| desugar(expr, loc)).next().unwrap();

            let binder = extract_binder(binder_or_val.0.clone(), binder_or_val.1.clone())?;

            let ty = fold_ty(binder.clone(), ty)?;

            TypedBinding {
                name: Some(binder.0),
                ty: Some(ty),
                val: None,
            }
        }
        item => match prior {
            BindingPriority::NameTyVal => {
                let binder = extract_binder(FixAst(item), loc.clone())?;
                TypedBinding {
                    name: Some(binder.0),
                    ty: None,
                    val: None,
                }
            }
            BindingPriority::TyNameVal => {
                let ty = desugar(FixAst(item), loc.clone())?;
                TypedBinding {
                    name: None,
                    ty: Some(ty),
                    val: None,
                }
            }
            BindingPriority::ValNameTy => {
                let val = desugar(FixAst(item), loc.clone())?;
                TypedBinding {
                    name: None,
                    ty: None,
                    val: Some(val),
                }
            }
        },
    };
    Ok(binding)
}

pub fn with_zero_span(x: NamedExpr) -> SpannedNamedExpr {
    SpannedExpr((x.0.fmap(|x| Box::new(with_zero_span(*x))), Vec::new()))
}

// TODO: We need a system to allow for partial type annotation
// TODO: We also need to keep the spans in the exprTree
pub fn desugar(expr: FixAst, loc: Vec<usize>) -> Result<SpannedNamedExpr, ExprError<Vec<usize>>> {
    use Ast::*;
    let new_expr = match expr.0 {
        Var(name) => ExprF::Var {
            idx: VarKind::Named(name),
        },
        Lit(lit) => match lit {
            LitKind::Number(n) => {
                let num = create_church_num(n);
                return Ok(with_zero_span(num));
            }
            LitKind::String(s) => {
                let lit = create_string(s);
                return Ok(with_zero_span(lit));
            }
        },
        // We dont know the type until we name resolve and we dont name resolve until we desugar
        // this even requires infer_type to work with named variables because we base our accessors
        // names
        // on the names of the type of the lhs
        Dot(_) => {
            todo!() //
        }
        App(items) => {
            let mut items = iter_with_loc(items, loc.clone());
            let base = items.next().unwrap();

            return items.fold(desugar(base.0, base.1), |acc, item| {
                Ok(SpannedExpr((
                    ExprF::App {
                        func: Box::new(acc?),
                        arg: Box::new(desugar(item.0.clone(), item.1.clone())?),
                    },
                    loc.clone(),
                )))
            });
        }

        Lambda(items) => {
            let mut items = iter_with_loc(items, loc.clone()).rev();
            let base = items.next().unwrap();

            return items.fold(desugar(base.0, base.1), |acc, item| {
                let binding =
                    extract_binding(item.0.clone(), item.1.clone(), BindingPriority::NameTyVal)?;

                Ok(SpannedExpr((
                    ExprF::Lambda {
                        name: binding
                            .name
                            .clone()
                            .map(|x| VarKind::Named(x.0))
                            .unwrap_or(VarKind::Idx(())),
                        param_ty: Box::new(
                            binding.ty.ok_or(ExprError::MissingType(item.1.clone()))?,
                        ),
                        body: Box::new(acc?),
                    },
                    loc.clone(),
                )))
            });
        }

        Pi(items) => {
            let mut items = iter_with_loc(items, loc.clone()).rev();
            let base = items.next().unwrap();

            return items.fold(desugar(base.0, base.1), |acc, item| {
                let binding =
                    extract_binding(item.0.clone(), item.1.clone(), BindingPriority::TyNameVal)?;

                Ok(SpannedExpr((
                    ExprF::Pi {
                        name: binding
                            .name
                            .clone()
                            .map(|x| VarKind::Named(x.0))
                            .unwrap_or(VarKind::Idx(())),
                        param_ty: Box::new(binding.ty.ok_or(ExprError::MissingType(item.1))?),
                        ret_ty: Box::new(acc?),
                    },
                    loc.clone(),
                )))
            });
        }
        Tuple(items) => {
            let bindings = to_bindings(items, loc.clone());
            return create_tuple(bindings, loc.clone());
        }
        Sigma(items) => {
            let bindings = to_bindings(items, loc.clone());
            return create_sigma(bindings, loc.clone());
        }
        Pipe(items) => {
            let mut items = iter_with_loc(items, loc.clone());
            let base = items.next().unwrap();

            return items.fold(desugar(base.0, base.1), |acc, item| {
                Ok(SpannedExpr((
                    ExprF::App {
                        func: Box::new(acc?),
                        arg: Box::new(desugar(item.0.clone(), item.1.clone())?),
                    },
                    loc.clone(),
                )))
            });
        }
        Group(expr) => {
            return desugar(*expr, push_new(loc.clone(), 0));
        }
        Type => ExprF::Type,
        Unit => ExprF::Lambda {
            name: VarKind::Idx(()),
            param_ty: Box::new(SpannedExpr((ExprF::Type, loc.clone()))),
            body: Box::new(SpannedExpr((
                ExprF::Lambda {
                    name: VarKind::Idx(()),
                    param_ty: Box::new(SpannedExpr((
                        ExprF::Var {
                            idx: VarKind::Idx(0),
                        },
                        loc.clone(),
                    ))),
                    body: Box::new(SpannedExpr((
                        ExprF::Var {
                            idx: VarKind::Idx(0),
                        },
                        loc.clone(),
                    ))),
                },
                loc.clone(),
            ))),
        },
        Ann(_) => {
            let binding = extract_binding(expr, loc.clone(), BindingPriority::ValNameTy)?;
            return binding.val.ok_or(ExprError::MissingValue(loc.clone()));
        }
        Binding(_) => {
            let binding = extract_binding(expr, loc.clone(), BindingPriority::ValNameTy)?;
            return binding.val.ok_or(ExprError::MissingValue(loc.clone()));
        }

        Module(_deps, items) => {
            let loc_items = push_new(loc.clone(), 1);

            let bindings = to_bindings(items, loc_items.clone());

            return create_tuple(bindings.clone(), loc_items.clone());
        }
    };

    Ok(SpannedExpr((new_expr, loc.clone())))
}

fn to_bindings(
    items: Vec<FixAst>,
    loc: Vec<usize>,
) -> Vec<Spanned<Result<TypedBinding, ExprError<Vec<usize>>>>> {
    let items = iter_with_loc(items, loc.clone());
    items
        .map(|x| {
            (
                extract_binding(x.0, x.1.clone(), BindingPriority::ValNameTy),
                x.1,
            )
        })
        .collect()
}

#[derive(Debug, Clone)]
pub struct TypedBindingNoOption {
    name: Option<Spanned<String>>,
    ty: SpannedNamedExpr,
    val: SpannedNamedExpr,
}

// (R : Type) => (f : (x : A1) -> (y : A2 x) -> (z : A3 x y) -> R) => f value_x value_y value_z
// A1
// we could let it be so the later fields can refer to previous fields directly without self
// reference modules
pub fn create_tuple(
    items: Vec<Spanned<Result<TypedBinding, ExprError<Vec<usize>>>>>,
    loc: Vec<usize>,
) -> Result<SpannedNamedExpr, ExprError<Vec<usize>>> {
    let items_len = items.len();
    // the f in the function body
    let f_var = SpannedExpr((
        ExprF::Var {
            idx: VarKind::Idx(0 as usize),
        },
        loc.clone(),
    ));

    let items: Result<Vec<Spanned<TypedBindingNoOption>>, ExprError<Vec<usize>>> = items
        .into_iter()
        .map(|item| {
            let binding = item.0?;

            let fullbinding = TypedBindingNoOption {
                name: binding.name,
                ty: binding.ty.ok_or(ExprError::MissingType(item.1.clone()))?,
                val: binding.val.ok_or(ExprError::MissingValue(item.1.clone()))?,
            };

            Ok::<_, ExprError<Vec<usize>>>((fullbinding, item.1.clone()))
        })
        .collect();
    let items = items?;

    // creating a new list folding instead of map because dependency on previous values
    let items: Vec<Spanned<TypedBindingNoOption>> =
        items.clone().into_iter().fold(Vec::new(), |mut acc, item| {
            let binding = item.0;

            // folding the individual one
            let new_binding = acc.clone().into_iter().fold(
                (binding, item.1),
                |acc: Spanned<TypedBindingNoOption>, item: Spanned<TypedBindingNoOption>| {
                    let old = acc.0;
                    let loc = acc.1;
                    let item = item.0;

                    let ty = SpannedExpr((
                        ExprF::App {
                            func: Box::new(SpannedExpr((
                                ExprF::Lambda {
                                    name: VarKind::Idx(()),
                                    param_ty: Box::new(item.ty.clone()),
                                    body: Box::new(old.ty.clone()),
                                },
                                old.ty.0.1.clone(),
                            ))),
                            arg: Box::new(item.val.clone()),
                        },
                        old.ty.0.1,
                    ));

                    let val = SpannedExpr((
                        ExprF::App {
                            func: Box::new(SpannedExpr((
                                ExprF::Lambda {
                                    name: VarKind::Idx(()),
                                    param_ty: Box::new(item.ty.clone()),
                                    body: Box::new(old.val.clone()),
                                },
                                old.val.0.1.clone(),
                            ))),
                            arg: Box::new(item.val.clone()),
                        },
                        old.val.0.1,
                    ));

                    (
                        TypedBindingNoOption {
                            name: old.name,
                            ty,
                            val,
                        },
                        loc.clone(),
                    )
                },
            );
            acc.push(new_binding);
            acc
        });

    let body = items.clone().into_iter().fold(Ok(f_var), |acc, item| {
        let acc = acc?;
        let binding = item.0;

        Ok(SpannedExpr((
            ExprF::App {
                func: Box::new(acc),
                arg: Box::new(binding.val.clone()),
            },
            item.1.clone(),
        )))
    })?;

    // the R as the return type of f
    let r_type = SpannedExpr((
        ExprF::Var {
            idx: VarKind::Idx(items_len),
        },
        loc.clone(),
    ));

    let f_type = items
        .clone()
        .into_iter()
        .rev()
        .fold(Ok(r_type.clone()), |acc, item| {
            let binding = item.0;

            Ok(SpannedExpr((
                ExprF::Pi {
                    name: binding
                        .name
                        .map(|x| VarKind::Named(x.0))
                        .ok_or(ExprError::MissingBinder(item.1.clone()))?,

                    param_ty: Box::new(binding.ty.clone()),
                    ret_ty: Box::new(acc?),
                },
                item.1.clone(),
            )))
        })?;

    Ok(SpannedExpr((
        ExprF::Lambda {
            name: VarKind::Idx(()),
            param_ty: Box::new(SpannedExpr((ExprF::Type, loc.clone()))),
            body: Box::new(SpannedExpr((
                ExprF::Lambda {
                    name: VarKind::Idx(()),
                    param_ty: Box::new(f_type),
                    body: Box::new(body),
                },
                loc.clone(),
            ))),
        },
        loc.clone(),
    )))
}

// (R : Type) -> (f : (x : A1) -> (y : A2 x) -> (z : A3 x y) -> R) -> R
pub fn create_sigma(
    items: Vec<Spanned<Result<TypedBinding, ExprError<Vec<usize>>>>>,
    loc: Vec<usize>,
) -> Result<SpannedNamedExpr, ExprError<Vec<usize>>> {
    // the R as the return type of f
    let r_type = SpannedExpr((
        ExprF::Var {
            idx: VarKind::Idx(items.len()),
        },
        loc.clone(),
    ));

    let f_type = items
        .into_iter()
        .rev()
        .fold(Ok(r_type.clone()), |acc, item| {
            let binding = item.0?;

            Ok(SpannedExpr((
                ExprF::Pi {
                    name: binding
                        .name
                        .map(|x| VarKind::Named(x.0))
                        .ok_or(ExprError::MissingBinder(item.1.clone()))?,

                    param_ty: Box::new(binding.ty.ok_or(ExprError::MissingType(item.1.clone()))?),
                    ret_ty: Box::new(acc?),
                },
                item.1.clone(),
            )))
        })?;

    Ok(SpannedExpr((
        ExprF::Pi {
            name: VarKind::Idx(()),
            param_ty: Box::new(SpannedExpr((ExprF::Type, loc.clone()))),
            ret_ty: Box::new(SpannedExpr((
                ExprF::Pi {
                    name: VarKind::Idx(()),
                    param_ty: Box::new(f_type),
                    ret_ty: Box::new(SpannedExpr((
                        ExprF::Var {
                            idx: VarKind::Idx(1),
                        },
                        loc.clone(),
                    ))),
                },
                loc.clone(),
            ))),
        },
        loc.clone(),
    )))
}
