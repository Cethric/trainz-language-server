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

        // Root container (lines 0 to 8)
        assert!(ranges.iter().any(|r| r.start_line == 0 && r.end_line == 8));
        // Nested container (lines 3 to 7)
        assert!(ranges.iter().any(|r| r.start_line == 3 && r.end_line == 7));
    }

    #[test]
    fn test_soup_folding_range_end_character() {
        let code = "container\n{\n    key \"value\"\n}";
        let pairs = gs_parser::soup::parse_soup(code).unwrap();
        let soup = gs_ast::soup::process::process_soup_ast(
            pairs,
            code,
            std::path::Path::new(""),
            &vec![],
            &vec![],
        );
        let ranges = crate::soup::soup_folding_range(&soup);

        for range in &ranges {
            println!(
                "Found soup range: {}:{} - {}:{}",
                range.start_line,
                range.start_character.unwrap_or(0),
                range.end_line,
                range.end_character.unwrap_or(0)
            );
        }

        // container starts at line 0, ends at line 3.
        // Line 3 content is "}"
        // '}' is at index 0. end_character should be 1.
        let container_range = ranges
            .iter()
            .find(|r| r.start_line == 0)
            .expect("Container range not found");
        assert_eq!(container_range.end_line, 3);
        assert_eq!(container_range.end_character, Some(1));
    }
}
