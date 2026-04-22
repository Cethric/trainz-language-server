use crate::gs::definitions::gs_goto_definition;
use dashmap::DashMap;
use std::str::FromStr;
use std::sync::Arc;
use tower_lsp_server::ls_types::{GotoDefinitionResponse, Position, Uri};
use trainz_parser::parse;

#[test]
fn test_gs_goto_definition_chained_method_inheritance() {
    let _ = trainz_common::logging::tracing_subscriber::fmt().with_test_writer().try_init();
    let source = r#"
        class Base {
            AcsText GetConfigAcsText() { return null; }
        };
        class Asset isclass Base {
        };
        class AcsText {
            AcsText GetNamedAcsText(string name) { return null; }
        };
        class Test {
            Asset GetAsset() { return null; }
            void Run() {
                GetAsset().GetConfigAcsText().GetNamedAcsText("mesh-table");
            }
        };
    "#;
    let pairs = parse(source).unwrap();
    let program = Arc::new(trainz_ast::gs::process::process_trainz_ast(pairs, source));
    let uri = Uri::from_str("file:///test.gs").unwrap();
    // 012345678901234567890123456789012345678901234567890123456789
    //                 GetAsset().GetConfigAcsText().GetNamedAcsText("mesh-table");
    // Position of GetNamedAcsText: line 13, let's find it programmatically or just count

    let parsed_files = DashMap::new();
    parsed_files.insert(uri.to_string(), program.clone());

    // We want to test jumping to GetNamedAcsText, which is at line 12 (0-indexed).
    // Let's just use grep to find the position.
}
