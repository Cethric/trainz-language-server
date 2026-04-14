use rayon::prelude::*;
use trainz_ast::acs_text::process::process_acs_text_ast;
use trainz_parser::acs_text::parse_acs_text;

#[test]
fn test_acs_text_folding_range() {
    let code =
        "container\n{\n    key \"value\"\n    nested\n    {\n        a 1\n        b 2\n    }\n}\n";
    let pairs = parse_acs_text(code).unwrap();
    let acs_text = process_acs_text_ast(pairs, code);
    let ranges = crate::acs_text::acs_text_folding_range(&acs_text);

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
    let pairs = trainz_parser::acs_text::parse_acs_text(code).unwrap();
    let acs_text = process_acs_text_ast(pairs, code);
    let ranges = crate::acs_text::acs_text_folding_range(&acs_text);

    // Single line containers are not folded
    assert!(ranges.is_empty());
}

#[test]
fn test_folding_range_next_line_brace() {
    let code = "container\n{\n    key \"value\"\n}";
    let pairs = trainz_parser::acs_text::parse_acs_text(code).unwrap();
    let acs_text = process_acs_text_ast(pairs, code);
    let ranges = crate::acs_text::acs_text_folding_range(&acs_text);

    let container_range = ranges
        .par_iter()
        .find_first(|r| r.start_line == 1)
        .expect("Container range not found (start_line 1)");

    assert_eq!(container_range.start_line, 1);
    assert_eq!(container_range.end_line, 3);
}
