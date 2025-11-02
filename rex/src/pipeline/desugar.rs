
use functor_derive::Functor;
use num_bigint::BigUint;

use crate::{data::{ast::{LitKind, NormalSugarExpr, SugarExpr}, expr::{ExprError, ExprF, GExpr, NamedExpr, Spanned, SpannedExpr, SpannedNamedExpr, VarKind}}, helper::push_new};

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


pub fn iter_with_loc<T: Clone>(items: Vec<T>, loc: Vec<usize>) -> impl DoubleEndedIterator<Item = (T, Vec<usize>)> + Clone {
    items.into_iter().enumerate().map(move |(new_loc, expr)| (expr, push_new(loc.clone(), new_loc)))

}

pub fn extract_binder(
    expr: NormalSugarExpr,
    loc: Vec<usize>,
) -> Result<FullBinder, ExprError<Spanned<NormalSugarExpr>>> {
    use SugarExpr::*;
    match expr.0 {
        Group(inner) => extract_binder(*inner, push_new(loc, 0)),
        Var(x) => Ok((((x, loc.clone())), Vec::new())),
        App(items) => {

            let mut items = iter_with_loc(items, loc.clone());

            let name = items.next().unwrap();

            let name_as_str = match name.0.0 {
                Var(name) => name,
                _ => {
                    return Err(ExprError::InvalidBinderName(name));
                }
            };

            let items = items.map(|(expr, loc)| {
                extract_binding(expr, loc.clone(), true).map(|x| (x, loc.clone()))
            });

            Ok(((name_as_str, name.1.clone()), items.collect::<Result<_,_>>()?))
        }
        _ => Err(ExprError::InvalidBinder((
            expr.clone(),
            loc.clone(),
        ))),
    }
}


pub fn extract_binding(
    expr: NormalSugarExpr,
    loc: Vec<usize>,
    is_lambda: bool,
) -> Result<TypedBinding, ExprError<Spanned<NormalSugarExpr>>> {
    use SugarExpr::*;
    let binding = match expr.0 {
        // first item is name second is val
        Group(inner) => extract_binding(*inner, push_new(loc.clone(), 0), is_lambda)?,
        Binding(items) => {
            let mut items = iter_with_loc(items, loc.clone());
                

            let named = items.next().unwrap();

            let val = items
                .map(|(expr, loc)| desugar(expr, loc))
                .last()
                .unwrap();

            match named.0.0 {
                Ann(items) => {
                    let mut items = iter_with_loc(items, named.1.clone());
                    let binder_expr = items.next().unwrap();

                    let ty = items
                        .map(|(expr, i)| desugar(expr, i))
                        .next()
                        .unwrap();

                    let binder = extract_binder(binder_expr.0.clone(), binder_expr.1.clone())?;

                    // converting 'succ (n : Nat) : Nat' into 'succ: (n: Nat) -> Nat'
                    let ty = binder.1.clone().into_iter().clone().rev().fold(ty, |acc, item| {
                        Ok(SpannedExpr((
                            ExprF::Pi {
                                name: item.0
                                    .name
                                    .clone()
                                    .map(|x| VarKind::Named(x.0))
                                    .unwrap_or(VarKind::Idx(())),
                                param_ty: Box::new(item.0.ty.clone().ok_or(
                                    ExprError::InvalidBinderParam(
                                        binder_expr.clone()
                                    ),
                                )?),
                                ret_ty: Box::new(acc?),
                            },
                            item.1
                        )))
                    })?;

                    let val = binder.1.into_iter().clone().rev().fold(val, |acc, item| {
                        Ok(SpannedExpr((
                            ExprF::Lambda {
                                name: item.0
                                    .name
                                    .clone()
                                    .map(|x| VarKind::Named(x.0))
                                    .unwrap_or(VarKind::Idx(())),
                                param_ty: Box::new(item.0.ty.clone().ok_or(
                                    ExprError::InvalidBinderParam(binder_expr.clone()),
                                )?),
                                body: Box::new(acc?),
                            },
                            item.1
                        )))
                    })?;

                    TypedBinding {
                        name: Some(binder.0),
                        ty: Some(ty),
                        val: Some(val),
                    }                }
                _ => {
                    let binder = extract_binder(named.0.clone(), named.1.clone())?;

                    let val = binder.1.into_iter().clone().rev().fold(val, |acc, item| {
                        Ok(SpannedExpr((
                            ExprF::Lambda {
                                name: item.0
                                    .name
                                    .clone()
                                    .map(|x| VarKind::Named(x.0))
                                    .unwrap_or(VarKind::Idx(())),
                                param_ty: Box::new(item.0.ty.clone().ok_or(
                                    ExprError::InvalidBinderParam(named.clone()),
                                )?),
                                body: Box::new(acc?),
                            },
                            item.1
                        )))
                    })?;

                    TypedBinding {
                        name: Some(binder.0),
                        ty: None,
                        val: Some(val),
                    }                }
            }
        }
        Ann(items) => {
            let mut items = iter_with_loc(items, loc.clone());
            let binder_or_val = items.next().unwrap();


            let ty = items
                .map(|(expr, loc)| {
                    desugar(expr, loc)
                })
                .next()
                .unwrap()?;

            let binder = extract_binder(binder_or_val.0.clone(), binder_or_val.1.clone())?;

            let ty = binder.1.into_iter().clone().fold(Ok(ty), |acc, item| {
                Ok(SpannedExpr((
                    ExprF::Pi {
                        name: item.0
                            .name
                            .clone()
                            .map(|x| VarKind::Named(x.0))
                            .unwrap_or(VarKind::Idx(())),
                        param_ty: Box::new(item.0.ty.clone().ok_or(ExprError::MissingType(binder_or_val.clone()))?),
                        ret_ty: Box::new(acc?),
                    },
                    item.1
                )))
            });

            TypedBinding {
                name: Some(binder.0),
                ty: Some(ty?),
                val: None,
            }        }
        item => {
            if is_lambda {
                let binder = extract_binder(NormalSugarExpr(item), loc.clone())?;
                TypedBinding {
                    name: Some(binder.0),
                    ty: None,
                    val: None,
                }            } else {
                let ty = desugar(NormalSugarExpr(item), loc.clone())?;
                TypedBinding {
                    name: None,
                    ty: Some(ty),
                    val: None,
                }            }
        }
    };
    Ok(binding)
}

pub fn with_zero_span(x: NamedExpr) -> SpannedNamedExpr {
    SpannedExpr((x.0.fmap(|x| Box::new(with_zero_span(*x))), Vec::new()))
}


// TODO: We need a system to allow for partial type annotation
// TODO: We also need to keep the spans in the exprTree
pub fn desugar(
    expr: NormalSugarExpr,
    loc: Vec<usize>,
) -> Result<SpannedNamedExpr, ExprError<Spanned<NormalSugarExpr>>> {
    use SugarExpr::*;
    let new_expr = match expr.0 {
        Var(name) => ExprF::Var {
            idx: VarKind::Named(name),
        },
        Lit(lit) => match lit {
            LitKind::Number(n) => {
                let num = create_church_num(n);
                return Ok(with_zero_span(num));
            }
            _ => todo!(),
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

                let binding = extract_binding(item.0.clone(), item.1.clone(), true)?;

                Ok(SpannedExpr((
                    ExprF::Lambda {
                        name: binding
                            .name
                            .clone()
                            .map(|x| VarKind::Named(x.0))
                            .unwrap_or(VarKind::Idx(())),
                        param_ty: Box::new(binding.ty.ok_or(ExprError::MissingType(item.clone()))?),
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

                let binding = extract_binding(item.0.clone(), item.1.clone(), false)?;

                Ok(SpannedExpr((
                    ExprF::Pi {
                        name: binding
                            .name
                            .clone()
                            .map(|x| VarKind::Named(x.0))
                            .unwrap_or(VarKind::Idx(())),
                        param_ty: Box::new(binding.ty
                            .ok_or(ExprError::MissingType(item))?),
                        ret_ty: Box::new(acc?),
                    },
                    loc.clone(),
                )))
            });
        }
        Tuple(items) => return create_tuple(items, loc.clone()),
        Sigma(items) => return create_sigma(items, loc.clone()),
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
        Ann(items) => {
            let mut items = iter_with_loc(items, loc);

            let val = items.next().expect("ann should have at least two items");

            return desugar(val.0, val.1);
        }
        Binding(items) => {

            let mut items: Vec<_> = iter_with_loc(items, loc).collect();
            let item = items.pop().expect("binder should have at least two items");

            return desugar(item.0, item.1);
        }

        Module(_deps, items, self_dep) => {
            println!("TODO: only implicit dependency on the module itself is handled {loc:?}");

            let loc_items = push_new(loc.clone(), 1);

            // TODO: Both tuple and sigma compile the body twice which is not what we want
            let inner = create_tuple(items.clone(), loc_items.clone());
            let inner_ty = create_sigma(items.clone(), loc_items.clone())?;

            let items = iter_with_loc(items, loc_items.clone());


            let res =
            // HACK: we wrap in lambdas until we can do proper field access
                if self_dep {
                    // a1 => a2 => ... => an => A
                    let expr = items.rev().fold(
                        inner,
                        |acc, item| {

                            let binding = extract_binding(item.0.clone(), item.1.clone(), true)?;

                            Ok(SpannedExpr((
                                ExprF::Lambda {
                                    name: binding
                                        .name
                                        .clone()
                                        .map(|x| VarKind::Named(x.0))
                                        .unwrap_or(VarKind::Idx(())),
                                    param_ty: Box::new(binding.ty.ok_or(
                                        ExprError::MissingType(item.clone()),
                                    )?),
                                    body: Box::new(acc?),
                                },
                                loc_items.clone(),
                            )))
                        },
                    );

                    // A -> A
                    Ok(SpannedExpr((
                    ExprF::Lambda { 
                        name: VarKind::Idx(()),
                        param_ty: Box::new(inner_ty),
                        body: Box::new(SpannedExpr((ExprF::App {
                            func: Box::new(SpannedExpr(
                                (ExprF::Var {
                                    idx: VarKind::Idx(0)},
                                loc_items.clone()))),
                            arg: Box::new(expr?) }, loc_items.clone())))},
                    loc_items.clone()
                )))

                } else {
                    inner
                };
            return res;
        }
    };
    Ok(SpannedExpr((new_expr, loc.clone())))
}

// (R : Type) => (f : (x : A1) -> (y : A2 x) -> (z : A3 x y) -> R) => f x y z
pub fn create_tuple(
    items: Vec<NormalSugarExpr>,
    loc: Vec<usize>,
) -> Result<SpannedNamedExpr, ExprError<Spanned<NormalSugarExpr>>> {
    let items_len = items.len();
    // the f in the function body
    let f_var = SpannedExpr((
        ExprF::Var {
            idx: VarKind::Idx(0 as usize),
        },
        loc.clone(),
    ));

    let items = iter_with_loc(items, loc.clone());

    let body = items.clone().fold(Ok(f_var), |acc, item| {
            let binding = extract_binding(item.0.clone(), item.1.clone(), false)?;


            Ok(SpannedExpr((
                ExprF::App {
                    func: Box::new(acc?),
                    arg: Box::new(
                        binding
                            .val
                            .ok_or(ExprError::MissingValue(item.clone()))?,
                    ),
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


    let f_type =
            items.rev()
            .fold(Ok(r_type.clone()), |acc, item| {
                let binding = extract_binding(item.0.clone(), item.1.clone(), false)?;


                Ok(SpannedExpr((
                    ExprF::Pi {
                        name: binding
                            .name
                            .map(|x| VarKind::Named(x.0))
                            .ok_or(ExprError::MissingBinder(item.clone()))?,

                        param_ty: Box::new(
                            binding
                                .ty
                                .ok_or(ExprError::MissingType(item.clone()))?,
                        ),
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
    items: Vec<NormalSugarExpr>,
    loc: Vec<usize>,
) -> Result<SpannedNamedExpr, ExprError<Spanned<NormalSugarExpr>>> {
    // the R as the return type of f
    let r_type = SpannedExpr((
        ExprF::Var {
            idx: VarKind::Idx(items.len()),
        },
        loc.clone(),
    ));

    let items = iter_with_loc(items, loc.clone());

    let f_type = items.rev().fold(
        Ok(r_type.clone()),
        |acc, item| {
            let binding = extract_binding(item.0.clone(), item.1.clone(), false)?;

            Ok(SpannedExpr((
                ExprF::Pi {
                    name: binding
                        .name
                        .map(|x| VarKind::Named(x.0))
                        .ok_or(ExprError::MissingBinder(item.clone()))?,

                    param_ty: Box::new(
                        binding
                            .ty
                            .ok_or(ExprError::MissingType(item.clone()))?,
                    ),
                    ret_ty: Box::new(acc?),
                },
                item.1.clone(),
            )))
        },
    )?;
    Ok(SpannedExpr((
        ExprF::Pi {
            name: VarKind::Idx(()),
            param_ty: Box::new(SpannedExpr((ExprF::Type, loc.clone()))),
            ret_ty: Box::new(SpannedExpr((
                ExprF::Pi {
                    name: VarKind::Idx(()),
                    param_ty: Box::new(f_type),
                    ret_ty: Box::new(r_type),
                },
                loc.clone(),
            ))),
        },
        loc.clone(),
    )))
}

