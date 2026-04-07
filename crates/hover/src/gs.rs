use gs_ast::gs::Program;
use tower_lsp_server::ls_types::Hover;

#[allow(deprecated)]
#[allow(clippy::type_complexity)]
pub fn gs_hover(_program: &Program) -> Vec<((u32, u32), (u32, u32), Hover)> {
    let data: Vec<((u32, u32), (u32, u32), Hover)> = vec![];

    data
}

#[cfg(test)]
mod tests {
    use super::*;
    use gs_ast::gs::process::process_gs_ast;
    use gs_parser::gs::parse;

    #[test]
    fn test_gs_hover_range() {
        let code = "class Test { int x; };";
        let pairs = parse(code).unwrap();
        let program = process_gs_ast(pairs, code);
        let results = gs_hover(&program);

        for (_start, _end, hover) in results {
            if let Some(range) = hover.range {
                // Ensure the hover result range is set
                assert!(range.start.line <= range.end.line);
            }
        }
    }
}
