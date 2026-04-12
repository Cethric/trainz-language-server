use tower_lsp_server::ls_types::Position;

pub trait HasRange {
    fn range(&self) -> tower_lsp_server::ls_types::Range;
}

#[tracing::instrument]
pub fn position_in_range(pos: Position, range: tower_lsp_server::ls_types::Range) -> bool {
    if pos.line < range.start.line || pos.line > range.end.line {
        return false;
    }
    if pos.line == range.start.line && pos.character < range.start.character {
        return false;
    }
    if pos.line == range.end.line && pos.character > range.end.character {
        return false;
    }
    true
}
