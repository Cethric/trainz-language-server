use gs_ast::gs::Include;
use tower_lsp_server::ls_types::{FoldingRange, FoldingRangeKind};

pub fn add_folding_range(
    range: tower_lsp_server::ls_types::Range,
    result: &mut Vec<FoldingRange>,
    collapsed_text: Option<String>,
) {
    if range.start.line < range.end.line {
        result.push(FoldingRange {
            start_line: range.start.line,
            start_character: Some(range.start.character),
            end_line: range.end.line,
            end_character: Some(range.end.character),
            kind: Some(FoldingRangeKind::Region),
            collapsed_text,
        });
    }
}

pub fn collect_include_folding_ranges(includes: &[Include], result: &mut Vec<FoldingRange>) {
    let mut i = 0;
    while i < includes.len() {
        let first_include = &includes[i];
        let mut last_include_end_line = if first_include.range.end.character == 0
            && first_include.range.end.line > first_include.range.start.line
        {
            first_include.range.end.line - 1
        } else {
            first_include.range.end.line
        };
        let mut j = i + 1;
        while j < includes.len() {
            let next_include = &includes[j];
            if next_include.range.start.line <= last_include_end_line + 1 {
                last_include_end_line = if next_include.range.end.character == 0
                    && next_include.range.end.line > next_include.range.start.line
                {
                    next_include.range.end.line - 1
                } else {
                    next_include.range.end.line
                };
                j += 1;
                continue;
            }
            break;
        }

        if j > i + 1 {
            let include = &includes[j - 1];
            let mut end_pos = include.path_range.unwrap_or(include.range).end;
            if end_pos.character == 0 && end_pos.line > first_include.range.start.line {
                end_pos.line -= 1;
            }
            let range = tower_lsp_server::ls_types::Range {
                start: first_include.range.start,
                end: end_pos,
            };
            add_folding_range(range, result, Some(String::from("includes ...")));
        }
        i = j;
    }
}
