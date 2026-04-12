use tower_lsp_server::ls_types::Hover;
use trainz_ast::gs::program::Program;

#[allow(deprecated)]
#[allow(clippy::type_complexity)]
#[tracing::instrument]
pub fn trainz_hover(_program: &Program) -> Vec<((u32, u32), (u32, u32), Hover)> {
    let data: Vec<((u32, u32), (u32, u32), Hover)> = vec![];

    data
}

#[cfg(test)]
mod tests {
    use super::*;
    use trainz_ast::gs::process::process_trainz_ast;
    use trainz_parser::gs::parse;

    #[test]
    fn test_trainz_hover_range() {
        let code = "class Test { int x; };";
        let pairs = parse(code).unwrap();
        let program = process_trainz_ast(pairs, code);
        let results = trainz_hover(&program);

        for (_start, _end, hover) in results {
            if let Some(range) = hover.range {
                // Ensure the hover result range is set
                assert!(range.start.line <= range.end.line);
            }
        }
    }
}
