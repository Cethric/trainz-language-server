use pest::iterators::Pair;
use pest::{Position as PestPosition, RuleType};
use tower_lsp_server::ls_types::{Position, Range};

pub fn pair_to_range<Rule: RuleType>(pair: &Pair<Rule>) -> Range {
    span_to_range(&pair.as_span())
}

pub fn span_to_range(span: &pest::Span) -> Range {
    pos_to_range(&span.start_pos(), &span.end_pos())
}

pub fn pos_to_range(start: &PestPosition, end: &PestPosition) -> Range {
    let (start_line, start_col) = start.line_col();
    let (end_line, end_col) = end.line_col();
    Range {
        start: Position {
            line: (start_line - 1) as u32,
            character: (start_col - 1) as u32,
        },
        end: Position {
            line: (end_line - 1) as u32,
            character: (end_col - 1) as u32,
        },
    }
}

pub fn range_is_inside_range(range: &Range, other: &Range) -> bool {
    range.start.line <= other.start.line
        && range.end.line >= other.end.line
        && range.start.character <= other.start.character
        && range.end.character >= other.end.character
}
