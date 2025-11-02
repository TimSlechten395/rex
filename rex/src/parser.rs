mod compile;
mod r#type;

pub use compile::*;

pub use r#type::*;

use crate::Compile;

impl Compile for NormalSugarExpr {
    type Output = NamedExpr;

    type Error = ExprError<Spanned<Self, Self::Span>>;

    type Span = Vec<usize>;

    fn run(self) -> Result<Spanned<Self::Output, Self::Span>, Self::Error> {
        todo!()
    }
}
