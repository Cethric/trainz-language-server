#[cfg(test)]
mod tests {
    use crate::soup::soup_folding_range;
    use gs_ast::soup::process::process_soup_ast;
    use gs_parser::soup::parse_soup;
    use std::path::Path;

    #[test]
    fn test_soup_folding_range() {
        let code = "container\n{\n    key \"value\"\n    nested\n    {\n        a 1\n        b 2\n    }\n}\n";
        let pairs = parse_soup(code).unwrap();
        let soup = process_soup_ast(pairs, code, Path::new(""), &vec![], &vec![]);
        let ranges = soup_folding_range(&soup);

        assert!(ranges.len() >= 2);

        // Root container (lines 0 to 9)
        assert!(ranges.iter().any(|r| r.start_line == 0 && r.end_line == 9));
        // Nested container (lines 3 to 8)
        assert!(ranges.iter().any(|r| r.start_line == 3 && r.end_line == 8));
    }
}
