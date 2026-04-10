#[cfg(test)]
mod tests {
    use rayon::prelude::*;
    use trainz_ast::soup::process::process_soup_ast;
    use trainz_parser::soup::parse_soup;

    #[test]
    fn test_soup_folding_range() {
        let code = "container\n{\n    key \"value\"\n    nested\n    {\n        a 1\n        b 2\n    }\n}\n";
        let pairs = parse_soup(code).unwrap();
        let soup = process_soup_ast(pairs, code);
        let ranges = crate::soup::soup_folding_range(&soup);

        // container starts at line 1, ends at line 8.
        let container_range = ranges
            .par_iter()
            .find_first(|r| r.start_line == 1)
            .expect("Container range not found");
        assert_eq!(container_range.end_line, 8);
        assert_eq!(container_range.end_character, Some(1));
    }

    #[test]
    fn test_folding_range_same_line_brace() {
        let code = "container { key \"value\" }";
        let pairs = trainz_parser::soup::parse_soup(code).unwrap();
        let soup = process_soup_ast(pairs, code);
        let ranges = crate::soup::soup_folding_range(&soup);

        // Single line containers are not folded
        assert!(ranges.is_empty());
    }

    #[test]
    fn test_folding_range_next_line_brace() {
        let code = "container\n{\n    key \"value\"\n}";
        let pairs = trainz_parser::soup::parse_soup(code).unwrap();
        let soup = process_soup_ast(pairs, code);
        let ranges = crate::soup::soup_folding_range(&soup);

        let container_range = ranges
            .par_iter()
            .find_first(|r| r.start_line == 1)
            .expect("Container range not found (start_line 1)");

        assert_eq!(container_range.start_line, 1);
        assert_eq!(container_range.end_line, 3);
    }
}
