use std::collections::HashMap;

use crate::data::expr::{Expr, GExpr, NamedExpr, Spanned, SpannedNamedExpr, VarKind};

pub struct Def {
    pub name: String,
    pub expr: SpannedNamedExpr,
}

pub type Defs = Vec<(String, SpannedNamedExpr)>;
