#[test]
fn test_for_panic() {
    let source = "for(i=0; i<10; i++);";
    let pairs = parser::gs::GsParser::parse(parser::gs::Rule::program, source);
    // actually, let's just run cargo test and see if any fail, or I can add a test in crates/ast/tests/
}
