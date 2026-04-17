use tower_lsp_server::ls_types::{Position, Range};

#[tracing::instrument(skip(position, range))]
pub fn is_in_range(position: Position, range: &Range) -> bool {
    if position.line < range.start.line || position.line > range.end.line {
        return false;
    }
    if position.line == range.start.line && position.character < range.start.character {
        return false;
    }
    if position.line == range.end.line && position.character > range.end.character {
        return false;
    }
    true
}
