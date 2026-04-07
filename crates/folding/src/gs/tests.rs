#[cfg(test)]
mod tests {
    use crate::gs::gs_folding_range;
    use gs_ast::gs::process::process_gs_ast;
    use gs_parser::gs::parse;

    #[test]
    fn test_gs_folding_range_minimal() {
        let code = "class Test {\n    void method() {\n        return;\n    }\n};\n";
        let pairs = parse(code).unwrap();
        let program = process_gs_ast(pairs, code);
        let ranges = gs_folding_range(&program);

        // Expect:
        // 1. Class body (lines 0-4)
        // 2. Method body (lines 1-3)
        assert!(ranges.len() >= 2);

        // Class body should be lines 0-4
        assert!(
            ranges.iter().any(|r| r.start_line == 0 && r.end_line == 4),
            "Class body folding range not found"
        );
        // Method body should be lines 1-3
        assert!(
            ranges.iter().any(|r| r.start_line == 1 && r.end_line == 3),
            "Method body folding range not found"
        );
    }

    #[test]
    fn test_gs_folding_range_complex() {
        let code = r#"class Test {
    void method() {
        if (true) {
            // inside if
        } else {
            // inside else
        }
        
        while (true) {
            // inside while
        }
        
        int i;
        for (i = 0; i < 10; i++) {
            // inside for
        }
        
        switch (x) {
            case 1: {
                // inside case
            }
            default: {
                // inside default
            }
        }
    }
};
"#;
        let pairs = parse(code).unwrap();
        let program = process_gs_ast(pairs, code);
        let ranges = gs_folding_range(&program);

        assert!(ranges.len() >= 1);

        // Method body (exact line numbers depend on formatting/parsing)
        assert!(ranges.iter().any(|r| r.start_line == 1));
    }

    #[test]
    fn test_gs_folding_range_stmt_blocks() {
        let code = r#"class Test {
    void method() {
        if (true) {
            // inside if
            int x;
        }
        while (true) {
            // inside while
            int y;
        }
        int i;
        for (i = 0; i < 10; i++) {
            // inside for
            int z;
        }
    }
};
"#;
        let pairs = parse(code).unwrap();
        let program = process_gs_ast(pairs, code);
        let ranges = gs_folding_range(&program);

        for range in &ranges {
            println!(
                "Found range: {}:{} - {}:{}",
                range.start_line,
                range.start_character.unwrap_or(0),
                range.end_line,
                range.end_character.unwrap_or(0)
            );
        }

        // 1. Method body (lines 1-18)
        assert!(ranges.iter().any(|r| r.start_line == 1));

        // 2. if block (lines 2-5)
        assert!(ranges.iter().any(|r| r.start_line == 2 && r.end_line == 5));

        // 3. while block (lines 6-9)
        assert!(ranges.iter().any(|r| r.start_line == 6 && r.end_line == 9));

        // 4. for block (lines 11-14)
        assert!(
            ranges
                .iter()
                .any(|r| r.start_line == 11 && r.end_line == 14)
        );

        // Let's check for at least 4 ranges.
        assert!(
            ranges.len() >= 4,
            "Expected at least 4 folding ranges (method + 3 stmt blocks), found {}",
            ranges.len()
        );
    }

    #[test]
    fn test_gs_folding_range_includes() {
        let code = "include \"common.gs\"\ninclude \"util.gs\"\ninclude \"lib.gs\"\n\nclass Test {\n    void method() {\n        // ...\n    }\n};\n";
        let pairs = parse(code).unwrap();
        let program = process_gs_ast(pairs, code);
        let ranges = gs_folding_range(&program);

        for range in &ranges {
            println!(
                "Found range: {}:{} - {}:{}",
                range.start_line,
                range.start_character.unwrap_or(0),
                range.end_line,
                range.end_character.unwrap_or(0)
            );
        }

        // Expect:
        // 1. Includes (lines 0-2)
        // 2. Method body (lines 5-7)

        assert!(
            ranges.iter().any(|r| r.start_line == 0 && r.end_line == 2),
            "Missing includes folding"
        );
    }

    #[test]
    fn test_gs_folding_range_class_body_explicit() {
        let code = r#"
class FirstClass {
    void first_method() {
        // Method 1 body
    }
};

class SecondClass {
    void second_method() {
        // Method 2 body
    }
};
"#;
        let pairs = parse(code).unwrap();
        let program = process_gs_ast(pairs, code);
        let ranges = gs_folding_range(&program);

        for range in &ranges {
            println!(
                "Found range: {}:{} - {}:{}",
                range.start_line,
                range.start_character.unwrap_or(0),
                range.end_line,
                range.end_character.unwrap_or(0)
            );
        }

        // 1. FirstClass body (lines 1-5)
        assert!(
            ranges.iter().any(|r| r.start_line == 1 && r.end_line == 5),
            "FirstClass body folding range not found"
        );

        // 2. SecondClass body (lines 7-11)
        assert!(
            ranges.iter().any(|r| r.start_line == 7 && r.end_line == 11),
            "SecondClass body folding range not found"
        );

        // 3. first_method body (lines 2-4)
        assert!(
            ranges.iter().any(|r| r.start_line == 2 && r.end_line == 4),
            "first_method body folding range not found"
        );

        // 4. second_method body (lines 8-10)
        assert!(
            ranges.iter().any(|r| r.start_line == 8 && r.end_line == 10),
            "second_method body folding range not found"
        );

        assert!(
            ranges.len() >= 4,
            "Expected at least 4 folding ranges, found {}",
            ranges.len()
        );
    }
}
