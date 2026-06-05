use anyhow::bail;

use crate::{
    Spanned,
    data::{
        ast::{self, SpannedFixAst},
        expr::Defs,
        tokens::{self, GToken, Token},
    },
};

pub fn push_new<T: Clone>(mut v: Vec<T>, elem: T) -> Vec<T> {
    v.push(elem);
    v
}

pub fn find_char(
    tokens: Vec<tokens::Spanned<Token>>,
    ast: Vec<SpannedFixAst>,
    expr: Defs,
    offset: usize,
) -> anyhow::Result<String> {
    // TODO: Should we use start or end? end is current winner
    let idx = match tokens.binary_search_by_key(&offset, |token| token.1.end) {
        Ok(idx) => idx,
        Err(idx) => idx,
    };

    let Some(token) = tokens.get(idx) else {
        bail!("Token was not in tokenstream");
    };

    // TODO: This is quite slow we might need to store an extra map or even store a list of
    // valid tokens?
    let ast_idx = map_index(&*tokens, idx);

    let ast_path = ast_idx.and_then(|x| {
        ast::search_list(ast.clone(), x)
            .into_iter()
            .find(|x| !x.is_empty())
    });

    let node = ast_path.clone().map(|x| ast::traverse_list(ast.clone(), x));

    let message = format!(
        "Found token {:?} with idx: {:?} in ast: {:?} (path: {:?})",
        token, ast_idx, node, ast_path,
    );

    Ok(message)
}

pub fn map_index<T>(v: &[tokens::Spanned<GToken<T>>], n: usize) -> Option<usize> {
    if n >= v.len() {
        return None; // out of bounds
    }
    if !matches!(v[n], (GToken::ValidToken(_), _)) {
        return None; // the element at n doesn't exist in the filtered Vec
    }

    // Count how many `Some`s before `n`
    let new_index = v[..n]
        .iter()
        .filter(|x| matches!(x, (GToken::ValidToken(_), _)))
        .count();
    Some(new_index)
}
