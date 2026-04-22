use crate::acs_text::acs_text_diagnostics;
use trainz_acs_text_validators::RulesRoot;
use trainz_ast::Range;
use trainz_ast::acs_text::{AcsText, KeyValuePair, Value};

#[test]
fn test_missing_kind() {
    let acs_text = AcsText {
        key_value_pairs: vec![],
        range: Range::default(),
        src: "".to_string(),
    };
    let graph = RulesRoot::new(std::collections::HashMap::new(), vec![]);
    let diagnostics = acs_text_diagnostics(&acs_text, &graph, None);
    assert!(!diagnostics.is_empty());
    assert_eq!(diagnostics[0].message, "'kind' is missing");
}

#[test]
fn test_unknown_kind() {
    let acs_text = AcsText {
        key_value_pairs: vec![KeyValuePair {
            key: "kind".to_string(),
            key_range: Range::default(),
            value: Some(Value::String("unknown".to_string(), Range::default())),
            range: Range::default(),
        }],
        range: Range::default(),
        src: "".to_string(),
    };
    let graph = RulesRoot::new(std::collections::HashMap::new(), vec![]);
    let diagnostics = acs_text_diagnostics(&acs_text, &graph, None);
    assert!(
        diagnostics
            .iter()
            .any(|d| d.message == "Unknown kind 'unknown'")
    );
}

#[test]
fn test_multiple_kinds() {
    let kvp = KeyValuePair {
        key: "kind".to_string(),
        key_range: Range::default(),
        value: Some(Value::String("test".to_string(), Range::default())),
        range: Range::default(),
    };
    let acs_text = AcsText {
        key_value_pairs: vec![kvp.clone(), kvp.clone()],
        range: Range::default(),
        src: "".to_string(),
    };
    let graph = RulesRoot::new(std::collections::HashMap::new(), vec![]);
    let diagnostics = acs_text_diagnostics(&acs_text, &graph, None);
    assert!(
        diagnostics
            .iter()
            .any(|d| d.message == "Multiple 'kind' tags found")
    );
}
