fn main() {
    let source = "class X { public void LinkPropertyValue(string pid) { inherited(pid); } };";
    let program = gs_parser::gs::parse(source).unwrap();
    let ast = gs_ast::gs::process::process_gs_ast(program, source);
    println!("{:#?}", ast.classes[0].methods[0]);
}
