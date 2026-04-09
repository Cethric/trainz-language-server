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

pub fn range_is_inside_range(container: &Range, item: &Range) -> bool {
    // Start comparison
    if item.start.line < container.start.line
        || (item.start.line == container.start.line
            && item.start.character < container.start.character)
    {
        return false;
    }
    // End comparison
    if item.end.line > container.end.line
        || (item.end.line == container.end.line && item.end.character > container.end.character)
    {
        return false;
    }
    true
}

pub fn clamp_range(container: &Range, item: Range) -> Range {
    let mut clamped = item;
    if item.start.line < container.start.line
        || (item.start.line == container.start.line
            && item.start.character < container.start.character)
    {
        clamped.start = container.start;
    }
    if item.end.line > container.end.line
        || (item.end.line == container.end.line && item.end.character > container.end.character)
    {
        clamped.end = container.end;
    }
    clamped
}
