use dashmap::DashMap;
use rayon::prelude::*;
use std::sync::Arc;
use tower_lsp_server::ls_types::{Location, ReferenceParams, Uri};
use tracing::trace;
use trainz_ast::find::position_in_range;
use trainz_ast::gs::find::find_id_at_position;
use trainz_ast::gs::program::Program;
use trainz_ast::gs::{Block, Expr, LoopBody, PostfixOp, Stmt, Type, TypeOrVoid};

#[tracing::instrument(skip(program, params, parsed_files))]
pub fn gs_find_references(
    program: Arc<Program>,
    params: ReferenceParams,
    parsed_files: &DashMap<String, Arc<Program>>,
) -> Option<Vec<Location>> {
    let position = params.text_document_position.position;
    let uri = params.text_document_position.text_document.uri;

    let mut target_identifier = None;
    let mut include_target_uri = None;

    // Check for includes first
    for include in &program.includes {
        if (position_in_range(position, include.range)
            || include
                .path_range
                .is_some_and(|r| position_in_range(position, r)))
            && let Some(path) = &include.path
        {
            include_target_uri = Some(Uri::from_file_path(path).unwrap());
            break;
        }
    }

    if include_target_uri.is_none()
        && let Some(id) = find_id_at_position(&program, position)
    {
        target_identifier = Some(id.name.to_string());
    }

    if let Some(target_uri) = include_target_uri {
        return Some(find_include_references(target_uri, parsed_files));
    }

    if let Some(target) = target_identifier {
        trace!("gs_find_references searching for {}", target);
        let locations: Vec<_> = parsed_files
            .par_iter()
            .flat_map(|entry| {
                let file_uri_str = entry.key();
                let file_program = entry.value();
                let file_uri = Uri::from_file_path(file_uri_str).unwrap();

                find_references_in_program(file_program, &target, &file_uri)
            })
            .collect();

        if !locations.is_empty() {
            return Some(locations);
        }
    }

    // If nothing found, try searching for the current file itself
    let current_file_references = find_include_references(uri, parsed_files);
    if !current_file_references.is_empty() {
        return Some(current_file_references);
    }

    None
}

fn find_include_references(
    target_file_uri: Uri,
    parsed_files: &DashMap<String, Arc<Program>>,
) -> Vec<Location> {
    let target_path = target_file_uri.to_file_path().unwrap_or_default();

    parsed_files
        .par_iter()
        .flat_map(|entry| {
            let file_uri_str = entry.key();
            let file_program = entry.value();
            let file_uri = Uri::from_file_path(file_uri_str).unwrap();

            let mut file_locations = vec![];
            for include in &file_program.includes {
                if let Some(include_path) = &include.path
                    && *include_path == target_path
                {
                    file_locations.push(Location {
                        uri: file_uri.clone(),
                        range: include.range,
                    });
                }
            }
            file_locations
        })
        .collect()
}

#[tracing::instrument(skip(program, target, uri))]
fn find_references_in_program(program: &Program, target: &str, uri: &Uri) -> Vec<Location> {
    program
        .classes
        .par_iter()
        .flat_map(|(_, cls)| {
            let mut locations = vec![];
            if cls.name.name == target {
                locations.push(Location {
                    uri: uri.clone(),
                    range: cls.name.range,
                });
            }
            for sup in &cls.superclasses {
                if sup.name == target {
                    locations.push(Location {
                        uri: uri.clone(),
                        range: sup.range,
                    });
                }
            }
            for field in cls.fields.values() {
                locations.extend(find_references_in_type(&field.ty, target, uri));
                if field.name.name == target {
                    locations.push(Location {
                        uri: uri.clone(),
                        range: field.name.range,
                    });
                }
                if let Some(init) = &field.initializer {
                    locations.extend(find_references_in_expr(init, target, uri));
                }
            }
            for ms in cls.methods.values() {
                for method in ms {
                    locations.extend(find_references_in_type_or_void(
                        &method.return_type,
                        target,
                        uri,
                    ));
                    if method.name.name == target {
                        locations.push(Location {
                            uri: uri.clone(),
                            range: method.name.range,
                        });
                    }
                    for param in &method.params {
                        locations.extend(find_references_in_type(&param.ty, target, uri));
                        if param.name.name == target {
                            locations.push(Location {
                                uri: uri.clone(),
                                range: param.name.range,
                            });
                        }
                    }
                    if let Some(body) = &method.body {
                        locations.extend(find_references_in_block(body, target, uri));
                    }
                }
            }
            locations
        })
        .collect()
}

#[tracing::instrument(skip(block, target, uri))]
fn find_references_in_block(block: &Block, target: &str, uri: &Uri) -> Vec<Location> {
    let mut locations = vec![];
    for stmt in &block.statements {
        locations.extend(find_references_in_stmt(stmt, target, uri));
    }
    locations
}

#[tracing::instrument(skip(stmt, target, uri))]
fn find_references_in_stmt(stmt: &Stmt, target: &str, uri: &Uri) -> Vec<Location> {
    let mut locations = vec![];
    match stmt {
        Stmt::Label(id, _, _) => {
            if id.name == target {
                locations.push(Location {
                    uri: uri.clone(),
                    range: id.range,
                });
            }
        }
        Stmt::Decl(decl) => {
            for name in &decl.names {
                if name.name == target {
                    locations.push(Location {
                        uri: uri.clone(),
                        range: name.range,
                    });
                }
            }
            for val in &decl.values {
                locations.extend(find_references_in_expr(val, target, uri));
            }
        }
        Stmt::Return(expr, _, _) => {
            if let Some(e) = expr {
                locations.extend(find_references_in_expr(e, target, uri));
            }
        }
        Stmt::Break(_, _) | Stmt::Continue(_, _) => {}
        Stmt::Goto(id, _, _) => {
            if id.name == target {
                locations.push(Location {
                    uri: uri.clone(),
                    range: id.range,
                });
            }
        }
        Stmt::Expr(expr) => {
            locations.extend(find_references_in_expr(expr, target, uri));
        }
        Stmt::If(if_stmt) => {
            locations.extend(find_references_in_expr(&if_stmt.cond, target, uri));
            locations.extend(find_references_in_block(&if_stmt.then_block, target, uri));
            if let Some(else_block) = &if_stmt.else_block {
                locations.extend(find_references_in_block(else_block, target, uri));
            }
        }
        Stmt::While(while_stmt) => {
            locations.extend(find_references_in_expr(&while_stmt.cond, target, uri));
            match &while_stmt.body {
                LoopBody::Empty(_) => {}
                LoopBody::Block(block) => {
                    locations.extend(find_references_in_block(block, target, uri))
                }
            }
        }
        Stmt::For(for_stmt) => {
            locations.extend(find_references_in_expr(&for_stmt.init.target, target, uri));
            locations.extend(find_references_in_expr(&for_stmt.init.value, target, uri));
            locations.extend(find_references_in_expr(&for_stmt.cond, target, uri));
            if let Some(step) = &for_stmt.step {
                locations.extend(find_references_in_expr(step, target, uri));
            }
            match &for_stmt.body {
                LoopBody::Empty(_) => {}
                LoopBody::Block(block) => {
                    locations.extend(find_references_in_block(block, target, uri))
                }
            }
        }
        Stmt::Wait(wait_stmt) => {
            locations.extend(find_references_in_block(&wait_stmt.body, target, uri));
        }
        Stmt::On(on_stmt) => {
            if let Some(id) = &on_stmt.identifier
                && id.name == target
            {
                locations.push(Location {
                    uri: uri.clone(),
                    range: id.range,
                });
            }
            locations.extend(find_references_in_block(&on_stmt.body, target, uri));
        }
        Stmt::Switch(switch_stmt) => {
            locations.extend(find_references_in_expr(&switch_stmt.expr, target, uri));
            for case in &switch_stmt.cases {
                locations.extend(find_references_in_expr(&case.value, target, uri));
                locations.extend(find_references_in_block(&case.body, target, uri));
            }
            if let Some(default) = &switch_stmt.default {
                locations.extend(find_references_in_block(default, target, uri));
            }
        }
        Stmt::Block(block) => {
            locations.extend(find_references_in_block(block, target, uri));
        }
    }
    locations
}

#[tracing::instrument(skip(expr, target, uri))]
fn find_references_in_expr(expr: &Expr, target: &str, uri: &Uri) -> Vec<Location> {
    let mut locations = vec![];
    match expr {
        Expr::Assign { left, right, .. }
        | Expr::LogicalOr { left, right, .. }
        | Expr::LogicalAnd { left, right, .. }
        | Expr::Equality { left, right, .. }
        | Expr::Comparison { left, right, .. }
        | Expr::Bitwise { left, right, .. }
        | Expr::BinaryMath { left, right, .. } => {
            locations.extend(find_references_in_expr(left, target, uri));
            locations.extend(find_references_in_expr(right, target, uri));
        }
        Expr::Unary { expr, .. } => {
            locations.extend(find_references_in_expr(expr, target, uri));
        }
        Expr::Postfix { expr, ops, .. } => {
            locations.extend(find_references_in_expr(expr, target, uri));
            for op in ops {
                match op {
                    PostfixOp::Deref(id) => {
                        if id.name == target {
                            locations.push(Location {
                                uri: uri.clone(),
                                range: id.range,
                            });
                        }
                    }
                    PostfixOp::Call(args, _) | PostfixOp::Index(args, _) => {
                        for arg in args {
                            locations.extend(find_references_in_expr(arg, target, uri));
                        }
                    }
                    PostfixOp::Unary(_, _) => {}
                }
            }
        }
        Expr::Literal(_) => {}
        Expr::IsClass(id) => {
            if id.name == target {
                locations.push(Location {
                    uri: uri.clone(),
                    range: id.range,
                });
            }
        }
        Expr::Cast { expr, ty, .. } => {
            locations.extend(find_references_in_expr(expr, target, uri));
            locations.extend(find_references_in_type(ty, target, uri));
        }
        Expr::NewObject { args, ty, .. } => {
            locations.extend(find_references_in_type(ty, target, uri));
            for arg in args {
                locations.extend(find_references_in_expr(arg, target, uri));
            }
        }
        Expr::NewArray { size, ty, .. } => {
            locations.extend(find_references_in_type(ty, target, uri));
            locations.extend(find_references_in_expr(size, target, uri));
        }
        Expr::Identifier(id) => {
            if id.name == target {
                locations.push(Location {
                    uri: uri.clone(),
                    range: id.range,
                });
            }
        }
        Expr::Grouped(expr, _) => {
            locations.extend(find_references_in_expr(expr, target, uri));
        }
    }
    locations
}

#[tracing::instrument(skip(ty, target, uri))]
fn find_references_in_type(ty: &Type, target: &str, uri: &Uri) -> Vec<Location> {
    let mut locations = vec![];
    match ty {
        Type::Named(id) => {
            if id.name == target {
                locations.push(Location {
                    uri: uri.clone(),
                    range: id.range,
                });
            }
        }
        Type::Array(inner, _) => {
            locations.extend(find_references_in_type(inner, target, uri));
        }
        _ => {}
    }
    locations
}

#[tracing::instrument(skip(ty, target, uri))]
fn find_references_in_type_or_void(ty: &TypeOrVoid, target: &str, uri: &Uri) -> Vec<Location> {
    match ty {
        TypeOrVoid::Type(t) => find_references_in_type(t, target, uri),
        TypeOrVoid::Void(_) => vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tower_lsp_server::ls_types::{
        Position, TextDocumentIdentifier, TextDocumentPositionParams,
    };
    use trainz_parser::gs::parse;

    #[test]
    fn test_gs_find_references_class() {
        let _ = trainz_common::logging::tracing_subscriber::fmt()
            .with_test_writer()
            .try_init();
        let source = "class MyClass { }; class Other { MyClass m; };";
        let pairs = parse(source).unwrap();
        let program = Arc::new(trainz_ast::gs::process::process_trainz_ast(pairs, source));
        let uri = Uri::from_file_path("/test.gs").unwrap();

        let parsed_files = DashMap::new();
        parsed_files.insert(
            uri.to_file_path().unwrap().to_string_lossy().to_string(),
            program.clone(),
        );

        let params = ReferenceParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: Position {
                    line: 0,
                    character: 6,
                }, // "MyClass"
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
            context: tower_lsp_server::ls_types::ReferenceContext {
                include_declaration: true,
            },
        };

        let result = gs_find_references(program, params, &parsed_files);
        assert!(result.is_some());
        let locations = result.unwrap();
        assert_eq!(locations.len(), 2); // Declaration and reference in "Other"
    }

    #[test]
    fn test_gs_find_references_include() {
        let _ = trainz_common::logging::tracing_subscriber::fmt()
            .with_test_writer()
            .try_init();
        let bar_path = "/path/to/Bar.gs";
        let source = format!("include \"{}\"\nclass Foo {{ }};", bar_path);
        let pairs = parse(&source).unwrap();
        let mut program = trainz_ast::gs::process::process_trainz_ast(pairs, &source);
        let path_range = program.includes[0].path_range;
        program.includes[0].path = Some(PathBuf::from(bar_path));
        let program = Arc::new(program);

        let uri = Uri::from_file_path("/test.gs").unwrap();
        let parsed_files = DashMap::new();
        parsed_files.insert(
            uri.to_file_path().unwrap().to_string_lossy().to_string(),
            program.clone(),
        );

        let params = ReferenceParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: path_range.unwrap().start, // inside include path
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
            context: tower_lsp_server::ls_types::ReferenceContext {
                include_declaration: true,
            },
        };

        let result = gs_find_references(program, params, &parsed_files);
        assert!(result.is_some());
        let locations = result.unwrap();
        assert_eq!(locations.len(), 1);
        assert_eq!(locations[0].uri, uri);
    }

    #[test]
    fn test_gs_find_references_current_file() {
        let _ = trainz_common::logging::tracing_subscriber::fmt()
            .with_test_writer()
            .try_init();
        let foo_path = "/path/to/Foo.gs";
        let bar_source = format!("include \"{}\"\nclass Bar {{ }};", foo_path);
        let bar_pairs = parse(&bar_source).unwrap();
        let mut bar_program = trainz_ast::gs::process::process_trainz_ast(bar_pairs, &bar_source);
        bar_program.includes[0].path = Some(PathBuf::from(foo_path));
        let bar_program = Arc::new(bar_program);

        let foo_source = "class Foo { };";
        let foo_pairs = parse(foo_source).unwrap();
        let foo_program = Arc::new(trainz_ast::gs::process::process_trainz_ast(
            foo_pairs, foo_source,
        ));

        let foo_uri = Uri::from_file_path(foo_path).unwrap();
        let bar_uri = Uri::from_file_path("/bar.gs").unwrap();

        let parsed_files = DashMap::new();
        parsed_files.insert(
            foo_uri
                .to_file_path()
                .unwrap()
                .to_string_lossy()
                .to_string(),
            foo_program.clone(),
        );
        parsed_files.insert(
            bar_uri
                .to_file_path()
                .unwrap()
                .to_string_lossy()
                .to_string(),
            bar_program.clone(),
        );

        let params = ReferenceParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: foo_uri.clone(),
                },
                position: Position {
                    line: 0,
                    character: 0,
                }, // Any position in Foo.gs
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
            context: tower_lsp_server::ls_types::ReferenceContext {
                include_declaration: true,
            },
        };

        let result = gs_find_references(foo_program, params, &parsed_files);
        assert!(result.is_some());
        let locations = result.unwrap();
        assert_eq!(locations.len(), 1);
        assert_eq!(locations[0].uri, bar_uri);
    }

    #[test]
    fn test_gs_find_references_multiple_includes() {
        let _ = trainz_common::logging::tracing_subscriber::fmt()
            .with_test_writer()
            .try_init();
        let target_path = "/path/to/Target.gs";
        let target_uri = Uri::from_file_path(target_path).unwrap();

        let source1 = format!("include \"{}\"\nclass A {{ }};", target_path);
        let mut program1 =
            trainz_ast::gs::process::process_trainz_ast(parse(&source1).unwrap(), &source1);
        program1.includes[0].path = Some(PathBuf::from(target_path));
        let program1 = Arc::new(program1);
        let uri1 = Uri::from_file_path("/a.gs").unwrap();

        let source2 = format!("include \"{}\"\nclass B {{ }};", target_path);
        let mut program2 =
            trainz_ast::gs::process::process_trainz_ast(parse(&source2).unwrap(), &source2);
        program2.includes[0].path = Some(PathBuf::from(target_path));
        let program2 = Arc::new(program2);
        let uri2 = Uri::from_file_path("/b.gs").unwrap();

        let target_source = "class Target {};";
        let target_program = Arc::new(trainz_ast::gs::process::process_trainz_ast(
            parse(target_source).unwrap(),
            target_source,
        ));

        let parsed_files = DashMap::new();
        parsed_files.insert(
            uri1.to_file_path().unwrap().to_string_lossy().to_string(),
            program1.clone(),
        );
        parsed_files.insert(
            uri2.to_file_path().unwrap().to_string_lossy().to_string(),
            program2.clone(),
        );

        let params = ReferenceParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: target_uri.clone(),
                },
                position: Position {
                    line: 0,
                    character: 0,
                },
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
            context: tower_lsp_server::ls_types::ReferenceContext {
                include_declaration: true,
            },
        };

        let result = gs_find_references(target_program, params, &parsed_files);
        assert!(result.is_some());
        let locations = result.unwrap();
        assert_eq!(locations.len(), 2);
        let uris: Vec<Uri> = locations.par_iter().map(|l| l.uri.clone()).collect();
        assert!(uris.contains(&uri1));
        assert!(uris.contains(&uri2));
    }

    #[test]
    fn test_gs_find_references_nested_includes() {
        let _ = trainz_common::logging::tracing_subscriber::fmt()
            .with_test_writer()
            .try_init();
        let a_path = "/path/to/A.gs";
        let b_path = "/path/to/B.gs";
        let c_path = "/path/to/C.gs";

        let a_uri = Uri::from_file_path(a_path).unwrap();
        let b_uri = Uri::from_file_path(b_path).unwrap();
        let c_uri = Uri::from_file_path(c_path).unwrap();

        // A includes B
        let a_source = format!("include \"{}\"", b_path);
        let mut a_program =
            trainz_ast::gs::process::process_trainz_ast(parse(&a_source).unwrap(), &a_source);
        a_program.includes[0].path = Some(PathBuf::from(b_path));
        let a_program = Arc::new(a_program);

        // B includes C and has a class
        let b_source = format!("include \"{}\"\nclass B {{}};", c_path);
        let mut b_program =
            trainz_ast::gs::process::process_trainz_ast(parse(&b_source).unwrap(), &b_source);
        b_program.includes[0].path = Some(PathBuf::from(c_path));
        let b_program = Arc::new(b_program);

        // C
        let c_source = "class C {};";
        let c_program = Arc::new(trainz_ast::gs::process::process_trainz_ast(
            parse(c_source).unwrap(),
            c_source,
        ));

        let parsed_files = DashMap::new();
        parsed_files.insert(
            a_uri.to_file_path().unwrap().to_string_lossy().to_string(),
            a_program.clone(),
        );
        parsed_files.insert(
            b_uri.to_file_path().unwrap().to_string_lossy().to_string(),
            b_program.clone(),
        );
        parsed_files.insert(
            c_uri.to_file_path().unwrap().to_string_lossy().to_string(),
            c_program.clone(),
        );

        // Find references to C from A (A includes B which includes C) - wait, from C
        let params_c = ReferenceParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: c_uri.clone() },
                position: Position {
                    line: 0,
                    character: 0,
                },
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
            context: tower_lsp_server::ls_types::ReferenceContext {
                include_declaration: true,
            },
        };

        let result_c = gs_find_references(c_program.clone(), params_c, &parsed_files);
        assert!(result_c.is_some());
        let locations_c = result_c.unwrap();
        assert_eq!(locations_c.len(), 1);
        assert_eq!(locations_c[0].uri, b_uri);

        // Find references to B (e.g. from B.gs itself at a position that is NOT an include and NOT an identifier)
        let params_b = ReferenceParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: b_uri.clone() },
                position: Position {
                    line: 1,
                    character: 10,
                }, // on line with class B {}; - character 10 is inside "{"
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
            context: tower_lsp_server::ls_types::ReferenceContext {
                include_declaration: true,
            },
        };

        let result_b = gs_find_references(b_program.clone(), params_b, &parsed_files);
        assert!(result_b.is_some());
        let locations_b = result_b.unwrap();
        assert_eq!(locations_b.len(), 1);
        assert_eq!(locations_b[0].uri, a_uri);
    }
}
