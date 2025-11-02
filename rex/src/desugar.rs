mod compile;
mod r#type;

use anyhow::bail;
pub use compile::*;
pub use r#type::*;

use crate::{Compile, Traverse, parser::NormalSugarExpr};

impl<A, B> Traverse for SpannedExpr<A, B> {
    type Span = Box<dyn Iterator<Item = usize>>;
    fn traverse(self, mut span: Self::Span) -> anyhow::Result<Box<Self>> {
        let current = span.next();
        match current {
            Some(cur) => match self.0.0 {
                ExprF::Var { .. } => {
                    bail!("reached var: no more nested expr")
                }
                ExprF::App { func, arg } => match cur {
                    0 => func.traverse(span),
                    1 => arg.traverse(span),
                    _ => bail!("reached app: index {cur} is invalid "),
                },
                ExprF::Lambda { param_ty, body, .. } => match cur {
                    0 => param_ty.traverse(span),
                    1 => body.traverse(span),
                    _ => bail!("reached lam: index {cur} is invalid "),
                },
                ExprF::Pi {
                    param_ty, ret_ty, ..
                } => match cur {
                    0 => param_ty.traverse(span),
                    1 => ret_ty.traverse(span),
                    _ => bail!("reached pi: index {cur} is invalid"),
                },
                ExprF::Type => bail!("reached type: no more nested expr"),
            },

            None => Ok(Box::new(self)),
        }
    }
}

impl Compile for NormalSugarExpr {
    type Output = NamedExpr;

    type Error = ExprError<crate::Spanned<Self, Self::Span>>;

    type Span = Vec<usize>;

    fn run(self) -> Result<crate::Spanned<Self::Output, Self::Span>, Self::Error> {
        todo!()
    }
}
