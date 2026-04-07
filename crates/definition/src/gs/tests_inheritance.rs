use crate::gs::definitions::gs_goto_definition;
use dashmap::DashMap;
use gs_parser::parse;
use std::str::FromStr;
use std::sync::Arc;
use tower_lsp_server::ls_types::{GotoDefinitionResponse, Position, Uri};

#[test]
fn test_gs_goto_definition_chained_method_inheritance() {
    let _ = env_logger::builder().is_test(true).try_init();
    let source = r#"
        class Base {
            Soup GetConfigSoup() { return null; }
        };
        class Asset isclass Base {
        };
        class Soup {
            Soup GetNamedSoup(string name) { return null; }
        };
        class Test {
            Asset GetAsset() { return null; }
            void Run() {
                GetAsset().GetConfigSoup().GetNamedSoup("mesh-table");
            }
        };
    "#;
    let pairs = parse(source).unwrap();
    let program = Arc::new(gs_ast::gs::process::process_gs_ast(pairs, source));
    let uri = Uri::from_str("file:///test.gs").unwrap();
    // 012345678901234567890123456789012345678901234567890123456789
    //                 GetAsset().GetConfigSoup().GetNamedSoup("mesh-table");
    // Position of GetNamedSoup: line 13, let's find it programmatically or just count

    let parsed_files = DashMap::new();
    parsed_files.insert(uri.to_string(), program.clone());

    // We want to test jumping to GetNamedSoup, which is at line 12 (0-indexed).
    // Let's just use grep to find the position.
}
