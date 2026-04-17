use trainz_ast::acs_text::value::Value;

#[tracing::instrument(skip(position, range))]
pub fn is_in_range(
    position: tower_lsp_server::ls_types::Position,
    range: &trainz_ast::Range,
) -> bool {
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

#[tracing::instrument(skip(value))]
pub fn get_value_range(value: &Value) -> trainz_ast::Range {
    value.range()
}
