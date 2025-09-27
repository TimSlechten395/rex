use rex::{lexer::Spanned, lexer::Token};
use tower_lsp_server::lsp_types::{Position, Range};

pub fn char_to_pos(text: &ropey::Rope, char: usize) -> (usize, usize) {
    let line = text.char_to_line(char);
    let line_start = text.line_to_char(line);
    let column = char - line_start;
    (line, column)
}

pub fn span_to_range(text: &ropey::Rope, span: std::ops::Range<usize>) -> Range {
    let (start_line, start_char) = char_to_pos(text, span.start);
    let (end_line, end_char) = char_to_pos(text, span.end);
    Range {
        start: Position {
            line: start_line as u32,
            character: start_char as u32,
        },
        end: Position {
            line: end_line as u32,
            character: end_char as u32,
        },
    }
}

pub fn map_index<E>(v: &[Spanned<Result<Token, E>>], n: usize) -> Option<usize> {
    if n >= v.len() {
        return None; // out of bounds
    }
    if !matches!(v[n], (Ok(Token::RealToken(_)), _)) {
        return None; // the element at n doesn't exist in the filtered Vec
    }

    // Count how many `Some`s before `n`
    let new_index = v[..n]
        .iter()
        .filter(|x| matches!(x, (Ok(Token::RealToken(_)), _)))
        .count();
    Some(new_index)
}
