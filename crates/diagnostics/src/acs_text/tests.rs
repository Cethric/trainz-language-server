use super::validator::*;
use rayon::prelude::*;
use tower_lsp_server::ls_types::DiagnosticSeverity;
use trainz_acs_text_validators::{ContainerValidator, Validators, load_validators};
use trainz_ast::acs_text::process::process_acs_text_ast;
use trainz_parser::acs_text::parse_acs_text;

#[test]
fn test_bool_kuid_filepath_types() {
    let temp_dir = std::env::current_dir()
        .unwrap()
        .join("temp_bool_kuid_filepath_test");
    if temp_dir.exists() {
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
    std::fs::create_dir_all(&temp_dir).unwrap();

    let content = r#"
test_container {
bool_key_0 0
bool_key_1 1
bool_key_error 2
kuid_key <kuid:1:2>
kuidbrowser_key <kuid:3:4>
filepath_key "some/path/file.txt"
}
"#;
    let pairs = parse_acs_text(content).unwrap();
    let acs_text = process_acs_text_ast(pairs, content);

    let container_txt = r#"
test_container
{
  kind "container"
  bool_key_0 { type "bool" }
  bool_key_1 { type "bool" }
  bool_key_error { type "bool" }
  kuid_key { type "kuid" }
  kuidbrowser_key { type "kuidbrowser" }
  filepath_key { type "filepath" }
}
"#;
    // In load_validators, 'is_container_style' checks for specific filenames.
    std::fs::write(temp_dir.join("container.txt"), container_txt).unwrap();

    let validators = load_validators(&temp_dir, None);
    let diagnostics = acs_text_diagnostics(&acs_text, &validators, None);

    let errors: Vec<_> = diagnostics
        .par_iter()
        .filter(|d| d.severity == Some(DiagnosticSeverity::ERROR))
        .collect();

    // We expect only one error for 'bool_key_error' because it has value '2'
    assert_eq!(
        errors.len(),
        1,
        "Expected exactly 1 error for bool_key_error, but found: {:?}",
        errors
    );
    assert!(errors[0].message.contains("bool_key_error"));
    assert!(
        errors[0]
            .message
            .contains("Expected 'bool', found 'numeric'")
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_case_insensitive_keys_in_acs_text() {
    let content = r#"
Signals {
0 { light -1 }
1 { light -1 }
2 { light -1 }
3 { light -1 }
}
"#;
    let pairs = parse_acs_text(content);
    assert!(pairs.is_ok());

    let pairs = pairs.unwrap();
    let acs_text = process_acs_text_ast(pairs, content);

    let temp_dir = std::env::current_dir()
        .unwrap()
        .join("temp_case_insensitive_test");
    if temp_dir.exists() {
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
    std::fs::create_dir_all(&temp_dir).unwrap();

    let container_txt = r#"
signals
{
  kind "container"
  array-element
  {
container-type0 "signal-possibility"
  }
  validation
  {
UniqueNames
  }
}

signal-possibility
{
  light
  {
type "numeric"
  }
}
"#;
    std::fs::write(temp_dir.join("container.txt"), container_txt).unwrap();

    let validators = load_validators(&temp_dir, None);
    let diagnostics = acs_text_diagnostics(&acs_text, &validators, None);

    // Check if there are any errors.
    // 1. "Signals" should match "signals" in container.txt
    // 2. "Light" should match "light" in signal-possibility
    let errors: Vec<_> = diagnostics
        .par_iter()
        .filter(|d| d.severity == Some(DiagnosticSeverity::ERROR))
        .collect();
    let warnings: Vec<_> = diagnostics
        .par_iter()
        .filter(|d| d.severity == Some(DiagnosticSeverity::WARNING))
        .collect();

    assert!(
        errors.is_empty(),
        "Expected no errors for case-insensitive keys, but found errors: {:?}",
        errors
    );
    assert!(
        warnings.is_empty(),
        "Expected no warnings (unknown key), but found: {:?}",
        warnings
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_case_insensitive_duplicates() {
    let temp_dir = std::env::current_dir()
        .unwrap()
        .join("temp_duplicates_case_insensitive_test");
    if temp_dir.exists() {
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
    std::fs::create_dir_all(&temp_dir).unwrap();

    let content2 = r#"
my_container {
KeyA "value"
keya "value"
}
"#;
    let pairs2 = parse_acs_text(content2).unwrap();
    let acs_text2 = process_acs_text_ast(pairs2, content2);

    let my_container_txt = r#"
my_container
{
  kind "container"
  validation
  {
UniqueNames
  }
  KeyA { type "string" }
}
"#;
    std::fs::write(temp_dir.join("my_container.txt"), my_container_txt).unwrap();

    let validators = load_validators(&temp_dir, None);
    let diagnostics = acs_text_diagnostics(&acs_text2, &validators, None);
    let has_duplicate_error = diagnostics.par_iter().any(|diag| {
        diag.message.to_lowercase().contains("duplicate key")
            && diag.message.to_lowercase().contains("'keya'")
    });

    assert!(
        has_duplicate_error,
        "Expected duplicate key error for case-insensitive 'KeyA' and 'keya', but found: {:?}",
        diagnostics
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_kind_txt_validation_valid_content() {
    let content = r#"
kind "my-kind"
key1 "value1"
key2 123
"#;
    let pairs = parse_acs_text(content).unwrap();
    let acs_text = process_acs_text_ast(pairs, content);

    let temp_dir = std::env::current_dir().unwrap().join("temp_kind_test");
    if temp_dir.exists() {
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
    std::fs::create_dir_all(&temp_dir).unwrap();

    let kind_txt = r#"
my-kind
{
  key1 { type "string" }
  key2 { type "numeric" }
}
"#;
    std::fs::write(temp_dir.join("kind.txt"), kind_txt).unwrap();

    let validators = load_validators(&temp_dir, None);
    let diagnostics = acs_text_diagnostics(&acs_text, &validators, None);

    let errors: Vec<_> = diagnostics
        .par_iter()
        .filter(|d| d.severity == Some(DiagnosticSeverity::ERROR))
        .collect();
    assert!(
        errors.is_empty(),
        "Expected no errors for kind-based validation, but found: {:?}",
        errors
    );

    std::fs::remove_dir_all(&temp_dir).unwrap();
}

#[test]
fn test_kind_txt_validation_invalid_content() {
    let invalid_content = r#"
kind "my-kind"
key1 123
"#;
    let pairs_invalid = parse_acs_text(invalid_content).unwrap();
    let acs_text_invalid = process_acs_text_ast(pairs_invalid, invalid_content);

    let temp_dir = std::env::current_dir()
        .unwrap()
        .join("temp_kind_test_invalid");
    if temp_dir.exists() {
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
    std::fs::create_dir_all(&temp_dir).unwrap();

    let kind_txt = r#"
my-kind
{
  key1 { type "string" }
}
"#;
    std::fs::write(temp_dir.join("kind.txt"), kind_txt).unwrap();

    let validators = load_validators(&temp_dir, None);
    let diagnostics_invalid = acs_text_diagnostics(&acs_text_invalid, &validators, None);

    let has_type_error = diagnostics_invalid
        .par_iter()
        .any(|d| d.message.contains("Invalid type for key 'key1'"));
    assert!(
        has_type_error,
        "Expected type error for key1, but found: {:?}",
        diagnostics_invalid
    );

    std::fs::remove_dir_all(&temp_dir).unwrap();
}

#[test]
fn test_numeric_keys_in_acs_text() {
    let content = r#"
signals {
0 {
    light -1
}
1 {
    light -1
}
2 {
    light -1
}
3 {
    light -1
}
}
"#;
    let pairs = parse_acs_text(content);
    assert!(
        pairs.is_ok(),
        "Failed to parse acs_text with numeric keys: {:?}",
        pairs.err()
    );

    let pairs = pairs.unwrap();
    let acs_text = process_acs_text_ast(pairs, content);

    // Create a temporary validation directory
    let temp_dir = std::env::current_dir()
        .unwrap()
        .join("temp_numeric_keys_test_unit");
    if temp_dir.exists() {
        std::fs::remove_dir_all(&temp_dir).unwrap();
    }
    std::fs::create_dir_all(&temp_dir).unwrap();

    let container_txt = r#"
signals
{
  kind "container"
  array-element
  {
container-type0 "signal-possibility"
  }
  validation
  {
UniqueNames
  }
}

signal-possibility
{
  light
  {
type "numeric"
  }
}
"#;
    std::fs::write(temp_dir.join("container.txt"), container_txt).unwrap();

    let validators = load_validators(&temp_dir, None);
    let diagnostics = acs_text_diagnostics(&acs_text, &validators, None);

    // Check if there are any errors. Numeric keys should be valid.
    let errors: Vec<_> = diagnostics
        .par_iter()
        .filter(|d| d.severity == Some(DiagnosticSeverity::ERROR))
        .collect();
    assert!(
        errors.is_empty(),
        "Expected no errors for numeric keys, but found: {:?}",
        errors
    );

    std::fs::remove_dir_all(&temp_dir).unwrap();
}

#[test]
fn test_sub_possibilities_validation() {
    let content = r#"
signals {
0 {
    light -1
}
4 {
    light -1
}
0 {
    light -1
}
}
"#;
    let pairs = parse_acs_text(content);
    assert!(pairs.is_ok());

    let pairs = pairs.unwrap();
    let acs_text = process_acs_text_ast(pairs, content);

    let temp_dir = std::env::current_dir()
        .unwrap()
        .join("temp_sub_possibilities_test");
    if temp_dir.exists() {
        std::fs::remove_dir_all(&temp_dir).unwrap();
    }
    std::fs::create_dir_all(&temp_dir).unwrap();

    let container_txt = r#"
signals
{
  kind "container"
  array-element
  {
container-type0 "signal-possibility"
  }
  validation
  {
SubPossibilities
  }
}

signal-possibility
{
  light
  {
type "numeric"
  }
}
"#;
    std::fs::write(temp_dir.join("container.txt"), container_txt).unwrap();

    let validators = load_validators(&temp_dir, None);
    let diagnostics = acs_text_diagnostics(&acs_text, &validators, None);

    // Check for duplicate key error.
    let has_duplicate_error = diagnostics.par_iter().any(|diag| {
        diag.message
            .contains("Duplicate key '0' in container 'signals'")
    });

    assert!(
        has_duplicate_error,
        "Expected duplicate key error for '0', but found: {:?}",
        diagnostics
    );

    std::fs::remove_dir_all(&temp_dir).unwrap();
}

#[test]
fn test_tuple_container_validation() {
    // Test that tuple containers (no braces for values) are also validated correctly.
    let content = r#"
signals {
0 { light -1 }
1 { light -1 }
2 { light -1 }
3 { light -1 }
}
"#;
    let pairs = parse_acs_text(content);
    assert!(pairs.is_ok());

    let pairs = pairs.unwrap();
    let acs_text = process_acs_text_ast(pairs, content);

    let temp_dir = std::env::current_dir().unwrap().join("temp_tuple_test");
    if temp_dir.exists() {
        std::fs::remove_dir_all(&temp_dir).unwrap();
    }
    std::fs::create_dir_all(&temp_dir).unwrap();

    let container_txt = r#"
signals
{
  kind "container"
  array-element
  {
container-type0 "signal-possibility"
  }
  validation
  {
SubPossibilities
  }
}

signal-possibility
{
  light
  {
type "numeric"
  }
}
"#;
    std::fs::write(temp_dir.join("container.txt"), container_txt).unwrap();

    let validators = load_validators(&temp_dir, None);
    let diagnostics = acs_text_diagnostics(&acs_text, &validators, None);

    let errors: Vec<_> = diagnostics
        .par_iter()
        .filter(|d| d.severity == Some(DiagnosticSeverity::ERROR))
        .collect();
    assert!(
        errors.is_empty(),
        "Expected no errors for valid tuple container, but found: {:?}",
        errors
    );

    std::fs::remove_dir_all(&temp_dir).unwrap();
}

#[test]
fn test_metadata_keys_ignored() {
    let content = r#"
my-container {
array-element {
    container-type0 "some-type"
}
subpossibilities {
    some-rule {
        type "string"
    }
}
validation {
    UniqueNames
}
top-level 1
known-rule "value"
}
"#;
    let pairs = parse_acs_text(content).unwrap();
    let acs_text = process_acs_text_ast(pairs, content);

    let temp_dir = std::env::current_dir()
        .unwrap()
        .join("temp_metadata_keys_test");
    if temp_dir.exists() {
        std::fs::remove_dir_all(&temp_dir).unwrap();
    }
    std::fs::create_dir_all(&temp_dir).unwrap();

    let container_txt = r#"
my-container
{
  kind "container"
  known-rule
  {
type "string"
  }
}
"#;
    std::fs::write(temp_dir.join("container.txt"), container_txt).unwrap();

    let validators = load_validators(&temp_dir, None);
    let diagnostics = acs_text_diagnostics(&acs_text, &validators, None);

    let unknown_key_errors: Vec<_> = diagnostics
        .par_iter()
        .filter(|diag| diag.message.contains("Unknown key"))
        .collect();

    assert!(
        unknown_key_errors.is_empty(),
        "Expected no unknown key errors for metadata keys, but found: {:?}",
        unknown_key_errors
    );

    std::fs::remove_dir_all(&temp_dir).unwrap();
}

#[test]
fn test_compulsory_optional_keys_do_not_error() {
    let content = r#"
    kind "test-container"
    required-key "present"
    obsolete-key "should-warn"
    "#;
    let pairs = parse_acs_text(content).unwrap();
    let acs_text = process_acs_text_ast(pairs, content);

    let temp_dir = std::env::current_dir()
        .unwrap()
        .join("temp_compulsory_test");
    if temp_dir.exists() {
        std::fs::remove_dir_all(&temp_dir).unwrap();
    }
    std::fs::create_dir_all(&temp_dir).unwrap();

    let container_txt = r#"
test-container
{
  kind "container"
  top-level 1
  required-key
  {
type "string"
compulsory 1
  }
  optional-key-zero
  {
type "string"
compulsory 0
  }
  optional-key-none
  {
type "string"
  }
  obsolete-key
  {
type "string"
obsolete-tag 1
  }
}
"#;
    std::fs::write(temp_dir.join("container.txt"), container_txt).unwrap();

    let validators = load_validators(&temp_dir, None);
    let diagnostics = acs_text_diagnostics(&acs_text, &validators, None);

    let missing_errors: Vec<_> = diagnostics
        .par_iter()
        .filter(|diag| {
            diag.message.contains("is missing") && diag.severity == Some(DiagnosticSeverity::ERROR)
        })
        .collect();

    assert!(
        missing_errors.is_empty(),
        "Expected no missing key errors for optional keys, but found: {:?}",
        missing_errors
    );

    std::fs::remove_dir_all(&temp_dir).unwrap();
}

#[test]
fn test_obsolete_key_generates_warning() {
    let content = r#"
    kind "test-container"
    required-key "present"
    obsolete-key "should-warn"
    "#;
    let pairs = parse_acs_text(content).unwrap();
    let acs_text = process_acs_text_ast(pairs, content);

    let temp_dir = std::env::current_dir()
        .unwrap()
        .join("temp_compulsory_test_obsolete");
    if temp_dir.exists() {
        std::fs::remove_dir_all(&temp_dir).unwrap();
    }
    std::fs::create_dir_all(&temp_dir).unwrap();

    let container_txt = r#"
test-container
{
  kind "container"
  top-level 1
  required-key
  {
type "string"
compulsory 1
  }
  obsolete-key
  {
type "string"
obsolete-tag 1
  }
}
"#;
    std::fs::write(temp_dir.join("container.txt"), container_txt).unwrap();

    let validators = load_validators(&temp_dir, None);
    let diagnostics = acs_text_diagnostics(&acs_text, &validators, None);

    let obsolete_warning = diagnostics.par_iter().find_first(|diag| {
        diag.message.contains("is obsolete/deprecated")
            && diag.severity == Some(DiagnosticSeverity::WARNING)
    });

    assert!(
        obsolete_warning.is_some(),
        "Expected warning for obsolete-key, but found: {:?}",
        diagnostics
    );

    std::fs::remove_dir_all(&temp_dir).unwrap();
}

#[test]
fn test_required_key_missing_validation() {
    let content = r#"
    kind "test-container"
    "#;
    let pairs = parse_acs_text(content).unwrap();
    let acs_text = process_acs_text_ast(pairs, content);

    let temp_dir = std::env::current_dir().unwrap().join("temp_required_test");
    if temp_dir.exists() {
        std::fs::remove_dir_all(&temp_dir).unwrap();
    }
    std::fs::create_dir_all(&temp_dir).unwrap();

    let container_txt = r#"
test-container
{
  kind "container"
  top-level 1
  required-key
  {
type "string"
compulsory 1
  }
}
"#;
    std::fs::write(temp_dir.join("container.txt"), container_txt).unwrap();

    let validators = load_validators(&temp_dir, None);
    let diagnostics = acs_text_diagnostics(&acs_text, &validators, None);

    // Should have an error for missing required-key
    let missing_error = diagnostics.par_iter().find_first(|diag| {
        diag.message
            .contains("Compulsory key 'required-key' is missing")
            && diag.severity == Some(DiagnosticSeverity::ERROR)
    });

    assert!(
        missing_error.is_some(),
        "Expected error for missing required-key, but found: {:?}",
        diagnostics
    );

    std::fs::remove_dir_all(&temp_dir).unwrap();
}

#[test]
fn test_thumbnails_element_validation() {
    let content = r#"
thumbnails {
0 {
    image   "icon/icon.jpg"
    width   240
    height  180
}
}
"#;
    let pairs = parse_acs_text(content).unwrap();
    let acs_text = process_acs_text_ast(pairs, content);

    let temp_dir = std::env::current_dir()
        .unwrap()
        .join("temp_thumbnails_test");
    if temp_dir.exists() {
        std::fs::remove_dir_all(&temp_dir).unwrap();
    }
    std::fs::create_dir_all(&temp_dir).unwrap();

    let container_txt = r#"
thumbnails
{
  kind "container"
  array-element
  {
container-type0 "thumbnails-element"
  }
}

thumbnails-element
{
  kind "container"
  image
  {
type "string"
  }
  width
  {
type "int"
  }
  height
  {
type "int"
  }
}
"#;
    std::fs::write(temp_dir.join("container.txt"), container_txt).unwrap();

    let validators = load_validators(&temp_dir, None);
    let diagnostics = acs_text_diagnostics(&acs_text, &validators, None);

    let has_type_error = diagnostics
        .par_iter()
        .any(|diag| diag.message.contains("Invalid type for key 'width'"));
    assert!(
        !has_type_error,
        "Found type error for 'width', but expected none: {:?}",
        diagnostics
    );

    std::fs::remove_dir_all(&temp_dir).unwrap();
}

#[test]
fn test_vector2_type_validation() {
    let content = r#"
my-container {
pos   0.5,0.5
}
"#;
    let pairs = parse_acs_text(content).unwrap();
    let acs_text = process_acs_text_ast(pairs, content);

    let temp_dir = std::env::current_dir().unwrap().join("temp_vector2_test");
    if temp_dir.exists() {
        std::fs::remove_dir_all(&temp_dir).unwrap();
    }
    std::fs::create_dir_all(&temp_dir).unwrap();

    let container_txt = r#"
my-container
{
  kind "container"
  pos
  {
type "vector2"
  }
}
"#;
    std::fs::write(temp_dir.join("container.txt"), container_txt).unwrap();

    let validators = load_validators(&temp_dir, None);
    let diagnostics = acs_text_diagnostics(&acs_text, &validators, None);

    let has_type_error = diagnostics
        .par_iter()
        .any(|diag| diag.message.contains("Invalid type for key 'pos'"));
    assert!(
        !has_type_error,
        "Found type error for 'pos' (vector2), but expected none: {:?}",
        diagnostics
    );

    std::fs::remove_dir_all(&temp_dir).unwrap();
}

#[test]
fn test_user_issue_string_table_validation() {
    let content = r#"string-table
{
  description                           "Unload train vehicles at current industry location."
  msg_error_industry_not_found          "Issue a 'Drive To' command prior to the 'Unload' command."
  driver_command_unload                 "Unload"
  tt_unload_at                          "Unload at $0"
}"#;
    let pairs = parse_acs_text(content).expect("Failed to parse acs_text");
    let acs_text = process_acs_text_ast(pairs, content);

    // Mock validator that has a "string-table" container but NOT marked as TagArray
    let mut validators = Validators::default();
    validators.containers.push(ContainerValidator {
        container_name: "string-table".to_string(),
        top_level: true, // Let's say it's top-level
        allow_any_key: true,
        ..Default::default()
    });

    let diagnostics = acs_text_diagnostics(&acs_text, &validators, None);

    // We expect NO "Unknown key" errors for 'description', etc., according to the requirement.
    for diag in &diagnostics {
        println!("Diagnostic: {}", diag.message);
    }

    let has_unknown_key_error = diagnostics
        .par_iter()
        .any(|diag| diag.message.contains("Unknown key"));

    assert!(
        !has_unknown_key_error,
        "Found unknown key error: {:?}",
        diagnostics
    );
}

#[test]
fn test_tag_array_validation() {
    let content = r#"
string-table {
type "string-entry"
Key1 {
    value "Value1"
}
Key2 {
    value "Value2"
}
Key1 {
    value "Value1 Duplicate"
}
}
"#;
    let pairs = parse_acs_text(content).unwrap();
    let acs_text = process_acs_text_ast(pairs, content);

    let temp_dir = std::env::current_dir().unwrap().join("temp_tag_array_test");
    if temp_dir.exists() {
        std::fs::remove_dir_all(&temp_dir).unwrap();
    }
    std::fs::create_dir_all(&temp_dir).unwrap();

    let container_txt = r#"
string-table
{
  kind "container"
  tag-array
  {
    kind "string-entry"
  }
  validation
  {
    UniqueNames
  }
}

string-entry
{
  kind "container"
  value
  {
type "string"
  }
}
"#;
    std::fs::write(temp_dir.join("container.txt"), container_txt).unwrap();

    let validators = load_validators(&temp_dir, None);
    let diagnostics = acs_text_diagnostics(&acs_text, &validators, None);

    // 1. Check for duplicate key error for "Key1"
    let has_duplicate_error = diagnostics
        .par_iter()
        .any(|diag| diag.message.contains("Duplicate key 'Key1'"));
    assert!(
        has_duplicate_error,
        "Expected duplicate key error for 'Key1', but found: {:?}",
        diagnostics
    );

    // 2. Add an invalid entry and check if it's validated against string-entry
    let content_invalid = r#"
string-table {
type "string-entry"
Key3 {
    invalid_key "Value"
}
}
"#;
    let pairs_invalid = parse_acs_text(content_invalid).unwrap();
    let acs_text_invalid = process_acs_text_ast(pairs_invalid, content_invalid);
    let validators = load_validators(&temp_dir, None);
    let diagnostics_invalid = acs_text_diagnostics(&acs_text_invalid, &validators, None);

    let has_unknown_key_error = diagnostics_invalid
        .par_iter()
        .any(|diag| diag.message.contains("Unknown key 'invalid_key'"));
    assert!(
        has_unknown_key_error,
        "Expected unknown key error for 'invalid_key' in string-entry, but found: {:?}",
        diagnostics_invalid
    );

    std::fs::remove_dir_all(&temp_dir).unwrap();
}

#[test]
fn test_tag_array_case_insensitivity() {
    let temp_dir = std::env::current_dir()
        .unwrap()
        .join("temp_tag_array_case_test");
    if temp_dir.exists() {
        std::fs::remove_dir_all(&temp_dir).unwrap();
    }
    std::fs::create_dir_all(&temp_dir).unwrap();

    let container_txt = r#"
string-table
{
  validation
  {
tagarray
  }
  type
  {
type "string"
  }
}
"#;
    std::fs::write(temp_dir.join("container.txt"), container_txt).unwrap();

    let content = r#"
string-table {
key "value"
}
"#;
    let pairs = parse_acs_text(content).unwrap();
    let acs_text = process_acs_text_ast(pairs, content);
    let validators = load_validators(&temp_dir, None);
    let diagnostics = acs_text_diagnostics(&acs_text, &validators, None);

    let has_error = diagnostics
        .par_iter()
        .any(|d| d.severity == Some(DiagnosticSeverity::ERROR));
    assert!(
        !has_error,
        "Unexpected errors with lowercase tagarray: {:?}",
        diagnostics
    );

    // Test mixed case TagArray in validation
    let container_txt_mixed = r#"
string-table
{
  validation
  {
TagArray
  }
  type
  {
type "string"
  }
}
"#;
    std::fs::write(temp_dir.join("container.txt"), container_txt_mixed).unwrap();
    let validators_mixed = load_validators(&temp_dir, None);
    let diagnostics_mixed = acs_text_diagnostics(&acs_text, &validators_mixed, None);
    let has_error_mixed = diagnostics_mixed
        .par_iter()
        .any(|d| d.severity == Some(DiagnosticSeverity::ERROR));
    assert!(
        !has_error_mixed,
        "Unexpected errors with mixed case TagArray: {:?}",
        diagnostics_mixed
    );

    // Test tag-array in validation
    let container_txt_tagarray = r#"
string-table
{
  validation
  {
tag-array
  }
  type
  {
type "string"
  }
}
"#;
    std::fs::write(temp_dir.join("container.txt"), container_txt_tagarray).unwrap();
    let validators_tagarray = load_validators(&temp_dir, None);
    let diagnostics_tagarray = acs_text_diagnostics(&acs_text, &validators_tagarray, None);
    let has_error_tagarray = diagnostics_tagarray
        .par_iter()
        .any(|d| d.severity == Some(DiagnosticSeverity::ERROR));
    assert!(
        !has_error_tagarray,
        "Unexpected errors with tag-array: {:?}",
        diagnostics_tagarray
    );

    // Check that tagarray itself is ignored in acs_text
    let content_meta = r#"
string-table {
key "value"
tagarray { }
tag-array { }
}
"#;
    let pairs_meta = parse_acs_text(content_meta).unwrap();
    let acs_text_meta = process_acs_text_ast(pairs_meta, content_meta);
    let diagnostics_meta = acs_text_diagnostics(&acs_text_meta, &validators, None);
    let has_unknown_key = diagnostics_meta
        .par_iter()
        .any(|d| d.message.contains("Unknown key"));
    assert!(
        !has_unknown_key,
        "tagarray/tag-array should be ignored as metadata keys in AcsText: {:?}",
        diagnostics_meta
    );

    std::fs::remove_dir_all(&temp_dir).unwrap();
}

#[test]
fn test_subpossibilities_validation() {
    let content = r#"
my-container {
kind "my-container"
required-key "value"
}
"#;
    let pairs = parse_acs_text(content).unwrap();
    let acs_text = process_acs_text_ast(pairs, content);

    let temp_dir = std::env::current_dir()
        .unwrap()
        .join("temp_subpossibilities_test");
    if temp_dir.exists() {
        std::fs::remove_dir_all(&temp_dir).unwrap();
    }
    std::fs::create_dir_all(&temp_dir).unwrap();

    let container_txt = r#"
my-container
{
  kind "container"
  kind
  {
type "string"
  }
  subpossibilities
  {
required-key
{
  compulsory 1
  type "string"
}
optional-key
{
  compulsory 0
  type "int"
}
  }
}
"#;
    std::fs::write(temp_dir.join("container.txt"), container_txt).unwrap();

    let validators = load_validators(&temp_dir, None);
    let diagnostics = acs_text_diagnostics(&acs_text, &validators, None);
    assert!(
        diagnostics.is_empty(),
        "Expected no errors for valid container, but found: {:?}",
        diagnostics
    );

    // Missing required key
    let content_missing = r#"
my-container {
kind "my-container"
}
"#;
    let pairs_missing = parse_acs_text(content_missing).unwrap();
    let acs_text_missing = process_acs_text_ast(pairs_missing, content_missing);
    let diagnostics_missing = acs_text_diagnostics(&acs_text_missing, &validators, None);

    let has_missing_key_error = diagnostics_missing.par_iter().any(|diag| {
        diag.message
            .contains("Compulsory key 'required-key' is missing")
    });
    assert!(
        has_missing_key_error,
        "Expected missing key error for 'required-key', but found: {:?}",
        diagnostics_missing
    );

    std::fs::remove_dir_all(&temp_dir).unwrap();
}

#[test]
fn test_top_level_inheritance() {
    let temp_dir = std::env::temp_dir().join("language-server-test-top-level");
    std::fs::create_dir_all(&temp_dir).unwrap();

    let kind_content = r#"
obsolete-track
{
  top-level 1
  icon track
  inherit
  {
itrack
base-asset
  }
  kind "structure"
  subpossibilities
  {
kind
{
  kind value
  type string
  compulsory 3.4
  default "track"
  filter "track"
  disabled 1
}
  }
}

itrack
{
  subpossibilities
  {
track-id
{
   type string
   compulsory 1
}
  }
}

base-asset
{
  subpossibilities
  {
asset-id
{
   type string
   compulsory 1
}
  }
}
"#;
    std::fs::write(temp_dir.join("kind.txt"), kind_content).unwrap();

    let acs_text_content = r#"
kind "obsolete-track"
asset-id "test-asset"
track-id "test-track"
"#;
    let pairs = parse_acs_text(acs_text_content).unwrap();
    let acs_text = process_acs_text_ast(pairs, acs_text_content);

    let validators = load_validators(&temp_dir, None);
    let diagnostics = acs_text_diagnostics(&acs_text, &validators, None);

    // Should NOT have any "Unknown key" errors if top_level validation worked
    let has_unknown_key = diagnostics
        .par_iter()
        .any(|d| d.message.contains("Unknown key"));
    assert!(
        !has_unknown_key,
        "Unexpected unknown key errors: {:?}",
        diagnostics
    );

    // Should NOT have error for 'asset-id' as it is inherited from base-asset
    let has_missing_asset_id = diagnostics
        .par_iter()
        .any(|d| d.message.contains("Compulsory key 'asset-id' is missing"));
    assert!(
        !has_missing_asset_id,
        "Unexpected error for 'asset-id': {:?}",
        diagnostics
    );

    // Should NOT have error for 'track-id' as it is inherited from itrack
    let has_missing_track_id = diagnostics
        .par_iter()
        .any(|d| d.message.contains("Compulsory key 'track-id' is missing"));
    assert!(
        !has_missing_track_id,
        "Unexpected error for 'track-id': {:?}",
        diagnostics
    );

    // Now test missing compulsory kind
    let acs_text_content_missing = r#"
kind "obsolete-track"
"#;
    let pairs_missing = parse_acs_text(acs_text_content_missing).unwrap();
    let acs_text_missing = process_acs_text_ast(pairs_missing, acs_text_content_missing);
    let diagnostics_missing = acs_text_diagnostics(&acs_text_missing, &validators, None);

    let has_missing_asset_id = diagnostics_missing
        .par_iter()
        .any(|d| d.message.contains("Compulsory key 'asset-id' is missing"));
    assert!(
        has_missing_asset_id,
        "Expected error for missing 'asset-id', but got: {:?}",
        diagnostics_missing
    );

    let has_missing_track_id = diagnostics_missing
        .par_iter()
        .any(|d| d.message.contains("Compulsory key 'track-id' is missing"));
    assert!(
        has_missing_track_id,
        "Expected error for missing 'track-id', but got: {:?}",
        diagnostics_missing
    );

    // 'kind' is marked as 'disabled 1' in obsolete-track subpossibilities, so it should not be checked for compulsory or it might even trigger an error if we implement disabled check.
    // In the example, it has compulsory 3.4.

    std::fs::remove_dir_all(&temp_dir).unwrap();
}

#[test]
fn test_subpossibilities_case_insensitivity() {
    let temp_dir = std::env::temp_dir().join("language-server-test-subpossibilities-case");
    if temp_dir.exists() {
        std::fs::remove_dir_all(&temp_dir).unwrap();
    }
    std::fs::create_dir_all(&temp_dir).unwrap();

    let kind_content = r#"
test-container
{
  SubPossibilities
  {
mandatory-key
{
  type string
  compulsory 1
}
Optional-Key
{
  type string
  compulsory 0
}
  }
}
"#;
    std::fs::write(temp_dir.join("kind.txt"), kind_content).unwrap();

    let validators = load_validators(&temp_dir, None);

    // 1. Test case-insensitive SubPossibilities in validator definition
    let acs_text_content = r#"
test-container
{
  mandatory-key "value"
}
"#;
    let pairs = parse_acs_text(acs_text_content).unwrap();
    let acs_text = process_acs_text_ast(pairs, acs_text_content);
    let diagnostics = acs_text_diagnostics(&acs_text, &validators, None);

    let has_error = diagnostics
        .par_iter()
        .any(|d| d.severity == Some(DiagnosticSeverity::ERROR));
    assert!(
        !has_error,
        "Unexpected errors with SubPossibilities: {:?}",
        diagnostics
    );

    // 2. Test missing compulsory key defined in SubPossibilities
    let acs_text_content_missing = r#"
test-container
{
  Optional-Key "value"
}
"#;
    let pairs_missing = parse_acs_text(acs_text_content_missing).unwrap();
    let acs_text_missing = process_acs_text_ast(pairs_missing, acs_text_content_missing);
    let diagnostics_missing = acs_text_diagnostics(&acs_text_missing, &validators, None);

    let has_missing_key = diagnostics_missing.par_iter().any(|d| {
        d.message
            .contains("Compulsory key 'mandatory-key' is missing")
    });
    assert!(
        has_missing_key,
        "Expected error for missing 'mandatory-key' defined in SubPossibilities: {:?}",
        diagnostics_missing
    );

    // 3. Test that SubPossibilities as a key in acs_text itself is ignored (not flagged as unknown)
    let acs_text_with_metadata = r#"
test-container
{
  mandatory-key "value"
  SubPossibilities { }
  subpossibilities { }
}
"#;
    let pairs_metadata = parse_acs_text(acs_text_with_metadata).unwrap();
    let acs_text_metadata = process_acs_text_ast(pairs_metadata, acs_text_with_metadata);
    let diagnostics_metadata = acs_text_diagnostics(&acs_text_metadata, &validators, None);

    let has_unknown_key = diagnostics_metadata
        .par_iter()
        .any(|d| d.message.contains("Unknown key"));
    assert!(
        !has_unknown_key,
        "SubPossibilities should be ignored as a metadata key in AcsText: {:?}",
        diagnostics_metadata
    );

    std::fs::remove_dir_all(&temp_dir).unwrap();
}

#[test]
fn test_category_class_validation() {
    let temp_dir = std::env::temp_dir().join("language-server-test-cat-class");
    if temp_dir.exists() {
        let _ = std::fs::remove_dir_all(&temp_dir);
    }
    std::fs::create_dir_all(&temp_dir).unwrap();

    let cat_class_content = r#"
Scenery "Scenery objects"
Track "Track objects"
"#;
    std::fs::write(temp_dir.join("category-class.txt"), cat_class_content).unwrap();

    let container_content = r#"
my_container {
kind "container"
top-level 1
category-class {
    type string
    validation IsValidCategoryClass
}
}
"#;
    std::fs::write(temp_dir.join("container.txt"), container_content).unwrap();

    let acs_text_content = r#"
my_container {
kind "my_container"
category-class "Scenery"
}
"#;
    let pairs = parse_acs_text(acs_text_content).unwrap();
    let acs_text = process_acs_text_ast(pairs, acs_text_content);

    let validators = load_validators(&temp_dir, None);
    let diagnostics = acs_text_diagnostics(&acs_text, &validators, None);

    assert!(
        diagnostics.is_empty(),
        "Unexpected diagnostics for valid category class: {:?}",
        diagnostics
    );

    let acs_text_content_invalid = r#"
my_container {
kind "my_container"
category-class "InvalidClass"
}
"#;
    let pairs_invalid = parse_acs_text(acs_text_content_invalid).unwrap();
    let acs_text_invalid = process_acs_text_ast(pairs_invalid, acs_text_content_invalid);
    let diagnostics_invalid = acs_text_diagnostics(&acs_text_invalid, &validators, None);
    println!("Diagnostics invalid: {:?}", diagnostics_invalid);

    let has_cat_class_error = diagnostics_invalid.par_iter().any(|d| {
        d.message
            .contains("Invalid value(s) 'InvalidClass' for key 'category-class'")
    });
    assert!(
        has_cat_class_error,
        "Expected category class error, but got: {:?}",
        diagnostics_invalid
    );

    // Test with "WAT" as requested by user
    let acs_text_content_wat = r#"
my_container {
kind "my_container"
category-class "WAT"
}
"#;
    let pairs_wat = parse_acs_text(acs_text_content_wat).unwrap();
    let acs_text_wat = process_acs_text_ast(pairs_wat, acs_text_content_wat);
    let diagnostics_wat = acs_text_diagnostics(&acs_text_wat, &validators, None);

    let has_wat_error = diagnostics_wat.par_iter().any(|d| {
        d.message
            .contains("Invalid value(s) 'WAT' for key 'category-class'")
    });
    assert!(
        has_wat_error,
        "Expected error for 'WAT' category class, but got: {:?}",
        diagnostics_wat
    );

    // Test Category Region
    let cat_region_content = r#"
FRA "France"
USA "United States"
"#;
    std::fs::write(temp_dir.join("category-region.txt"), cat_region_content).unwrap();

    let container_content_region = r#"
region_container {
category-region {
    type string
    validation IsValidCategoryRegion
}
}
"#;
    std::fs::write(temp_dir.join("container.txt"), container_content_region).unwrap();

    let acs_text_content_region = r#"
region_container {
category-region "FRA"
}
"#;
    let pairs_region = parse_acs_text(acs_text_content_region).unwrap();
    let acs_text_region = process_acs_text_ast(pairs_region, acs_text_content_region);
    let validators_region = load_validators(&temp_dir, None);
    let diagnostics_region = acs_text_diagnostics(&acs_text_region, &validators_region, None);
    assert!(
        diagnostics_region.is_empty(),
        "Unexpected diagnostics for valid category region: {:?}",
        diagnostics_region
    );

    let acs_text_content_fra_invalid = r#"
region_container {
category-region "GER"
}
"#;
    let pairs_fra_invalid = parse_acs_text(acs_text_content_fra_invalid).unwrap();
    let acs_text_fra_invalid =
        process_acs_text_ast(pairs_fra_invalid, acs_text_content_fra_invalid);
    let diagnostics_fra_invalid =
        acs_text_diagnostics(&acs_text_fra_invalid, &validators_region, None);
    let has_region_error = diagnostics_fra_invalid.par_iter().any(|d| {
        d.message
            .contains("Invalid value(s) 'GER' for key 'category-region'")
    });
    assert!(
        has_region_error,
        "Expected category region error, but got: {:?}",
        diagnostics_fra_invalid
    );

    // Test Category Era
    let cat_era_content = r#"
2000s "2000s"
2010s "2010s"
2020s "2020s"
"#;
    std::fs::write(temp_dir.join("category-era.txt"), cat_era_content).unwrap();

    let container_content_era = r#"
era_container {
kind "container"
top-level 1
category-era {
    type string
    validation IsValidCategoryEra
}
}
"#;
    std::fs::write(temp_dir.join("container.txt"), container_content_era).unwrap();

    let acs_text_content_era = r#"
era_container {
kind "era_container"
category-era "2000s;2010s;"
}
"#;
    let pairs_era = parse_acs_text(acs_text_content_era).unwrap();
    let acs_text_era = process_acs_text_ast(pairs_era, acs_text_content_era);
    let validators_era = load_validators(&temp_dir, None);
    let diagnostics_era = acs_text_diagnostics(&acs_text_era, &validators_era, None);
    assert!(
        diagnostics_era.is_empty(),
        "Unexpected diagnostics for valid category era: {:?}",
        diagnostics_era
    );

    let acs_text_content_era_invalid = r#"
era_container {
kind "era_container"
category-era "2000s;2030s;2010s;"
}
"#;
    let pairs_era_invalid = parse_acs_text(acs_text_content_era_invalid).unwrap();
    let acs_text_era_invalid =
        process_acs_text_ast(pairs_era_invalid, acs_text_content_era_invalid);
    let diagnostics_era_invalid =
        acs_text_diagnostics(&acs_text_era_invalid, &validators_era, None);
    let has_era_error = diagnostics_era_invalid.par_iter().any(|d| {
        d.message
            .contains("Invalid value(s) '2030s' for key 'category-era'")
    });
    assert!(
        has_era_error,
        "Expected category era error for '2030s', but got: {:?}",
        diagnostics_era_invalid
    );

    std::fs::remove_dir_all(&temp_dir).unwrap();
}

#[test]
fn test_disabled_keys_ignored() {
    let temp_dir = std::env::temp_dir().join("language-server-test-disabled");
    if temp_dir.exists() {
        std::fs::remove_dir_all(&temp_dir).unwrap();
    }
    std::fs::create_dir_all(&temp_dir).unwrap();

    let kind_content = r#"
test-container
{
  disabled-key
  {
type string
compulsory 1
disabled 1
  }
  normal-key
  {
type string
compulsory 1
  }
  top-level 1
}
"#;
    std::fs::write(temp_dir.join("kind.txt"), kind_content).unwrap();

    let validators = load_validators(&temp_dir, None);

    // 1. Test that disabled key does not trigger missing compulsory error (as child)
    let acs_text_content = r#"
test-container {
  normal-key "value"
}
"#;
    let pairs = parse_acs_text(acs_text_content).unwrap();
    let acs_text = process_acs_text_ast(pairs, acs_text_content);
    let diagnostics = acs_text_diagnostics(&acs_text, &validators, None);
    assert!(
        diagnostics.is_empty(),
        "Unexpected diagnostics for missing disabled key: {:?}",
        diagnostics
    );

    // 2. Test that disabled key does not trigger warning when present
    let acs_text_content_present = r#"
test-container {
  normal-key "value"
  disabled-key "whatever"
}
"#;
    let pairs_present = parse_acs_text(acs_text_content_present).unwrap();
    let acs_text_present = process_acs_text_ast(pairs_present, acs_text_content_present);
    let diagnostics_present = acs_text_diagnostics(&acs_text_present, &validators, None);
    assert!(
        diagnostics_present.is_empty(),
        "Unexpected diagnostics for present disabled key: {:?}",
        diagnostics_present
    );

    // 3. Test top-level disabled key
    let kind_content_top = r#"
test-kind
{
  disabled-top
  {
    type string
    compulsory 1
    disabled 1
  }
  normal-top
  {
    type string
    compulsory 1
  }
  top-level 1
}
"#;
    std::fs::write(temp_dir.join("kind.txt"), kind_content_top).unwrap();
    let validators_top = load_validators(&temp_dir, None);

    let acs_text_top = r#"
kind "test-kind"
normal-top "value"
"#;
    let pairs_top = parse_acs_text(acs_text_top).unwrap();
    let acs_text_obj_top = process_acs_text_ast(pairs_top, acs_text_top);
    let diagnostics_top = acs_text_diagnostics(&acs_text_obj_top, &validators_top, None);
    assert!(
        diagnostics_top.is_empty(),
        "Unexpected diagnostics for missing disabled top-level key: {:?}",
        diagnostics_top
    );

    std::fs::remove_dir_all(&temp_dir).unwrap();
}

#[test]
fn test_combobox() {
    let temp_dir = std::env::temp_dir().join("acs_text_combobox_test");
    if temp_dir.exists() {
        std::fs::remove_dir_all(&temp_dir).unwrap();
    }
    std::fs::create_dir_all(&temp_dir).unwrap();

    let container_txt = r#"
test-container
{
  kind "container"
  top-level 1
  combo { type "combobox" }
  list { type "listbox" }
  file { type "filepathedit" }
}
"#;
    std::fs::write(temp_dir.join("container.txt"), container_txt).unwrap();
    let validators = load_validators(&temp_dir, None);

    // 1. Valid case
    let valid_content = r#"
test-container {
kind "test-container"
combo "single_value"
}
"#;
    let pairs = parse_acs_text(valid_content).unwrap();
    let acs_text = process_acs_text_ast(pairs, valid_content);
    let diagnostics = acs_text_diagnostics(&acs_text, &validators, None);
    assert!(
        diagnostics.is_empty(),
        "Expected no diagnostics, found: {:?}",
        diagnostics
    );

    // 2. Case with semicolon (should be allowed, no warning)
    let invalid_content = r#"
test-container {
kind "test-container"
combo "val1;val2"
}
"#;
    let pairs_inv = parse_acs_text(invalid_content).unwrap();
    let acs_text_inv = process_acs_text_ast(pairs_inv, invalid_content);
    let diagnostics_inv = acs_text_diagnostics(
        &acs_text_inv,
        &validators,
        Some(&temp_dir.join("test.acs_text")),
    );
    println!("Diagnostics: {:?}", diagnostics_inv);
    assert!(
        diagnostics_inv.is_empty(),
        "Expected no diagnostics for combobox with semicolon, but found: {:?}",
        diagnostics_inv
    );

    std::fs::remove_dir_all(&temp_dir).unwrap();
}

#[test]
fn test_inline_nested_validator_diagnostics() {
    let temp_dir = std::env::temp_dir().join("acs_text_inline_nested_diagnostics_test");
    if temp_dir.exists() {
        std::fs::remove_dir_all(&temp_dir).unwrap();
    }
    std::fs::create_dir_all(&temp_dir).unwrap();

    let container_txt = r#"
example {
  kind "container"
  top-level 1
  nested {
    type "container"
    value {
      type "string"
      validation "IsValidValue"
    }
  }
}
"#;
    std::fs::write(temp_dir.join("container.txt"), container_txt).unwrap();
    std::fs::write(temp_dir.join("isvalidvalue.txt"), "val1 \"Value 1\"").unwrap();
    let validators = load_validators(&temp_dir, None);

    // 1. Valid case
    let valid_content = r#"
example {
kind "example"
nested {
  value "val1"
}
}
"#;
    let pairs = parse_acs_text(valid_content).unwrap();
    let acs_text = process_acs_text_ast(pairs, valid_content);
    let diagnostics = acs_text_diagnostics(&acs_text, &validators, None);
    assert!(
        diagnostics.is_empty(),
        "Expected no diagnostics for valid inline nested value, found: {:?}",
        diagnostics
    );

    // 2. Invalid case
    let invalid_content = r#"
example {
kind "example"
nested {
  value "invalid_val"
}
}
"#;
    let pairs_inv = parse_acs_text(invalid_content).unwrap();
    let acs_text_inv = process_acs_text_ast(pairs_inv, invalid_content);
    let diagnostics_inv = acs_text_diagnostics(&acs_text_inv, &validators, None);
    assert!(
        !diagnostics_inv.is_empty(),
        "Expected diagnostics for invalid inline nested value"
    );
    assert!(
        diagnostics_inv[0]
            .message
            .contains("not a valid IsValidValue")
            || diagnostics_inv[0]
                .message
                .contains("Invalid value(s) 'invalid_val' for key 'value'"),
        "Unexpected error message: {}",
        diagnostics_inv[0].message
    );

    std::fs::remove_dir_all(&temp_dir).unwrap();
}

#[test]
fn test_tagarray_nested_validator_diagnostics() {
    let temp_dir = std::env::temp_dir().join("acs_text_tagarray_nested_diagnostics_test");
    if temp_dir.exists() {
        std::fs::remove_dir_all(&temp_dir).unwrap();
    }
    std::fs::create_dir_all(&temp_dir).unwrap();

    let container_txt = r#"
extensions {
  kind "container"
  top-level 1
  tagarray {
    foo { type "string" }
  }
}
"#;
    std::fs::write(temp_dir.join("container.txt"), container_txt).unwrap();
    let validators = load_validators(&temp_dir, None);

    // 1. Valid case
    let valid_content = r#"
extensions {
kind "extensions"
my-ext {
  foo "bar"
}
}
"#;
    let pairs = parse_acs_text(valid_content).unwrap();
    let acs_text = process_acs_text_ast(pairs, valid_content);
    let diagnostics = acs_text_diagnostics(&acs_text, &validators, None);
    assert!(
        diagnostics.is_empty(),
        "Expected no diagnostics for valid nested tagarray, found: {:?}",
        diagnostics
    );

    // 2. Invalid case (invalid value type)
    let invalid_content = r#"
extensions {
kind "extensions"
my-ext {
  foo 123
}
}
"#;
    let pairs_inv = parse_acs_text(invalid_content).unwrap();
    let acs_text_inv = process_acs_text_ast(pairs_inv, invalid_content);
    let diagnostics_inv = acs_text_diagnostics(&acs_text_inv, &validators, None);
    assert!(
        !diagnostics_inv.is_empty(),
        "Expected diagnostics for invalid value in nested tagarray"
    );

    std::fs::remove_dir_all(&temp_dir).unwrap();
}

#[test]
fn test_array_element_nested_validator_diagnostics() {
    let temp_dir = std::env::temp_dir().join("acs_text_array_element_nested_diagnostics_test");
    if temp_dir.exists() {
        std::fs::remove_dir_all(&temp_dir).unwrap();
    }
    std::fs::create_dir_all(&temp_dir).unwrap();

    let container_txt = r#"
my-array {
  kind "container"
  top-level 1
  array-element {
    foo { type "string" }
  }
}
"#;
    std::fs::write(temp_dir.join("container.txt"), container_txt).unwrap();
    let validators = load_validators(&temp_dir, None);

    // 1. Valid case
    let valid_content = r#"
my-array {
kind "my-array"
0 {
  foo "bar"
}
}
"#;
    let pairs = parse_acs_text(valid_content).unwrap();
    let acs_text = process_acs_text_ast(pairs, valid_content);
    let diagnostics = acs_text_diagnostics(&acs_text, &validators, None);
    assert!(
        diagnostics.is_empty(),
        "Expected no diagnostics for valid nested array-element, found: {:?}",
        diagnostics
    );

    // 2. Invalid case (invalid value type)
    let invalid_content = r#"
my-array {
kind "my-array"
0 {
  foo 123
}
}
"#;
    let pairs_inv = parse_acs_text(invalid_content).unwrap();
    let acs_text_inv = process_acs_text_ast(pairs_inv, invalid_content);
    let diagnostics_inv = acs_text_diagnostics(&acs_text_inv, &validators, None);
    assert!(
        !diagnostics_inv.is_empty(),
        "Expected diagnostics for invalid value in nested array-element"
    );

    std::fs::remove_dir_all(&temp_dir).unwrap();
}

#[test]
fn test_listbox() {
    let temp_dir = std::env::temp_dir().join("acs_text_listbox_test");
    if temp_dir.exists() {
        std::fs::remove_dir_all(&temp_dir).unwrap();
    }
    std::fs::create_dir_all(&temp_dir).unwrap();

    let container_txt = r#"
test-container
{
  kind "container"
  top-level 1
  combo { type "combobox" }
  list { type "listbox" }
  file { type "filepathedit" }
}
"#;
    std::fs::write(temp_dir.join("container.txt"), container_txt).unwrap();
    let validators = load_validators(&temp_dir, None);

    let valid_content = r#"
test-container {
kind "test-container"
list "val1;val2;val3;"
}
"#;
    let pairs = parse_acs_text(valid_content).unwrap();
    let acs_text = process_acs_text_ast(pairs, valid_content);
    let diagnostics = acs_text_diagnostics(&acs_text, &validators, None);
    assert!(
        diagnostics.is_empty(),
        "Expected no diagnostics, found: {:?}",
        diagnostics
    );

    std::fs::remove_dir_all(&temp_dir).unwrap();
}

#[test]
fn test_filepathedit() {
    let temp_dir = std::env::temp_dir().join("acs_text_filepathedit_test");
    if temp_dir.exists() {
        std::fs::remove_dir_all(&temp_dir).unwrap();
    }
    std::fs::create_dir_all(&temp_dir).unwrap();

    let container_txt = r#"
test-container
{
  kind "container"
  top-level 1
  combo { type "combobox" }
  list { type "listbox" }
  file { type "filepathedit" }
}
"#;
    std::fs::write(temp_dir.join("container.txt"), container_txt).unwrap();

    // Create a dummy file for filepathedit test
    std::fs::write(temp_dir.join("existing_file.txt"), "hello").unwrap();

    let validators = load_validators(&temp_dir, None);

    // 1. Valid case
    let valid_content = r#"
test-container {
kind "test-container"
file "existing_file.txt"
}
"#;
    let pairs = parse_acs_text(valid_content).unwrap();
    let acs_text = process_acs_text_ast(pairs, valid_content);
    let diagnostics = acs_text_diagnostics(
        &acs_text,
        &validators,
        Some(&temp_dir.join("test.acs_text")),
    );
    assert!(
        diagnostics.is_empty(),
        "Expected no diagnostics, found: {:?}",
        diagnostics
    );

    // 2. Invalid case
    let invalid_content = r#"
test-container {
kind "test-container"
file "non_existent.txt"
}
"#;
    let pairs_inv = parse_acs_text(invalid_content).unwrap();
    let acs_text_inv = process_acs_text_ast(pairs_inv, invalid_content);
    let diagnostics_inv = acs_text_diagnostics(
        &acs_text_inv,
        &validators,
        Some(&temp_dir.join("test_inv.acs_text")),
    );
    let has_file_error = diagnostics_inv.par_iter().any(|d| {
        d.severity == Some(DiagnosticSeverity::ERROR) && d.message.contains("does not exist")
    });
    assert!(
        has_file_error,
        "Expected error for non-existent file in filepathedit, but found: {:?}",
        diagnostics_inv
    );

    std::fs::remove_dir_all(&temp_dir).unwrap();
}

#[test]
fn test_array_floatlist_validation() {
    let temp_dir = std::env::temp_dir().join("acs_text_floatlist_test");
    if temp_dir.exists() {
        std::fs::remove_dir_all(&temp_dir).unwrap();
    }
    std::fs::create_dir_all(&temp_dir).unwrap();

    let container_txt = r#"
test-container
{
  floats { type "floatlist" }
}
"#;
    std::fs::write(temp_dir.join("container.txt"), container_txt).unwrap();

    let validators = load_validators(&temp_dir, None);

    // 1. Array with only integers - should be valid for floatlist
    let int_content = r#"
test-container {
floats 1,2,3
}
"#;
    let pairs = parse_acs_text(int_content).unwrap();
    let acs_text = process_acs_text_ast(pairs, int_content);
    let diagnostics = acs_text_diagnostics(&acs_text, &validators, None);
    // Currently this might pass if "floatlist" is treated as "array" and no deeper check is done.
    // But let's see.
    assert!(
        diagnostics.is_empty(),
        "Expected no diagnostics for int array as floatlist, but found: {:?}",
        diagnostics
    );

    // 2. Array with at least one float - this is what the task is about
    let float_content = r#"
test-container {
floats 1,2.5,3
}
"#;
    let pairs_f = parse_acs_text(float_content).unwrap();
    let acs_text_f = process_acs_text_ast(pairs_f, float_content);
    let diagnostics_f = acs_text_diagnostics(&acs_text_f, &validators, None);

    assert!(
        diagnostics_f.is_empty(),
        "Expected no diagnostics for float array as floatlist, but found: {:?}",
        diagnostics_f
    );

    // 3. Test mismatch reporting
    let invalid_content = r#"
test-container {
floats "not-an-array"
}
"#;
    let pairs_i = parse_acs_text(invalid_content).unwrap();
    let acs_text_i = process_acs_text_ast(pairs_i, invalid_content);
    let diagnostics_i = acs_text_diagnostics(&acs_text_i, &validators, None);
    assert!(!diagnostics_i.is_empty());
    assert!(
        diagnostics_i[0]
            .message
            .contains("Expected 'floatlist', found 'string'")
    );

    // 4. Test float array when 'array' is expected
    let container_txt_2 = r#"
test-container-2
{
  arr { type "array" }
}
"#;
    std::fs::write(temp_dir.join("container2.txt"), container_txt_2).unwrap();
    let validators_2 = load_validators(&temp_dir, None);
    let float_content_2 = r#"
test-container-2 {
arr 1.5,2.5
}
"#;
    let pairs_f2 = parse_acs_text(float_content_2).unwrap();
    let acs_text_f2 = process_acs_text_ast(pairs_f2, float_content_2);
    let diagnostics_f2 = acs_text_diagnostics(&acs_text_f2, &validators_2, None);
    assert!(
        diagnostics_f2.is_empty(),
        "Expected no diagnostics for float array when 'array' type is expected, but found: {:?}",
        diagnostics_f2
    );

    std::fs::remove_dir_all(&temp_dir).unwrap();
}

#[test]
fn test_tuple_array_element_validation() {
    let content = r#"
my-tuple {
kind "my-tuple"
0 {
    value_str "hello"
}
1 {
    value_int 42
}
}
"#;
    let pairs = parse_acs_text(content).unwrap();
    let acs_text = process_acs_text_ast(pairs, content);

    let temp_dir = std::env::temp_dir().join(format!(
        "temp_tuple_element_test_{:?}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    if temp_dir.exists() {
        std::fs::remove_dir_all(&temp_dir).unwrap();
    }
    std::fs::create_dir_all(&temp_dir).unwrap();

    let container_txt = r#"
my-tuple
{
  kind "container"
  array-element
  {
container-type0 "string-elem"
container-type1 "int-elem"
  }
}

string-elem
{
  kind "container"
  value_str
  {
type "string"
compulsory 1.0
  }
}

int-elem
{
  kind "container"
  value_int
  {
type "int"
compulsory 1.0
  }
}
"#;
    std::fs::write(temp_dir.join("container.txt"), container_txt).unwrap();

    let validators = load_validators(&temp_dir, None);
    let diagnostics = acs_text_diagnostics(&acs_text, &validators, None);
    assert!(
        diagnostics.is_empty(),
        "Expected no errors for valid tuple, but found: {:?}",
        diagnostics
    );

    // Missing element
    let content_missing = r#"
my-tuple {
kind "my-tuple"
0 {
    value_str "hello"
}
}
"#;
    let pairs_missing = parse_acs_text(content_missing).unwrap();
    let acs_text_missing = process_acs_text_ast(pairs_missing, content_missing);
    let diagnostics_missing = acs_text_diagnostics(&acs_text_missing, &validators, None);
    assert!(
        diagnostics_missing
            .par_iter()
            .any(|d| d.message.contains("Missing tuple element '1'")),
        "Expected missing element error, but found: {:?}",
        diagnostics_missing
    );

    // Extraneous element
    let content_extra = r#"
my-tuple {
kind "my-tuple"
0 {
    value_str "hello"
}
1 {
    value_int 42
}
2 {
    value_int 99
}
}
"#;
    let pairs_extra = parse_acs_text(content_extra).unwrap();
    let acs_text_extra = process_acs_text_ast(pairs_extra, content_extra);
    let diagnostics_extra = acs_text_diagnostics(&acs_text_extra, &validators, None);
    assert!(
        diagnostics_extra
            .par_iter()
            .any(|d| d.message.contains("Index '2' out of bounds")),
        "Expected out of bounds error, but found: {:?}",
        diagnostics_extra
    );

    // Wrong element type
    let content_wrong_type = r#"
my-tuple {
kind "my-tuple"
0 {
    value_int 42
}
1 {
    value_int 42
}
}
"#;
    let pairs_wrong_type = parse_acs_text(content_wrong_type).unwrap();
    let acs_text_wrong_type = process_acs_text_ast(pairs_wrong_type, content_wrong_type);
    let diagnostics_wrong_type = acs_text_diagnostics(&acs_text_wrong_type, &validators, None);
    // Should have an error about missing compulsory key 'value_str' in 'string-elem'
    assert!(
        diagnostics_wrong_type
            .par_iter()
            .any(|d| d.message.contains("Compulsory key 'value_str' is missing")),
        "Expected missing compulsory key error for element 0, but found: {:?}",
        diagnostics_wrong_type
    );

    std::fs::remove_dir_all(&temp_dir).unwrap();
}

#[test]
fn test_three_element_tuple_validation() {
    let content = r#"
my-tuple {
kind "my-tuple"
0 { value_str "hello" }
1 { value_int 42 }
2 { value_str "world" }
}
"#;
    let pairs = parse_acs_text(content).unwrap();
    let acs_text = process_acs_text_ast(pairs, content);

    let temp_dir = std::env::temp_dir().join(format!(
        "temp_tuple_test_3_{:?}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&temp_dir).unwrap();

    let container_txt = r#"
my-tuple { kind "container" array-element { container-type0 "s" container-type1 "i" container-type2 "s" } }
s { kind "container" value_str { type "string" compulsory 1.0 } }
i { kind "container" value_int { type "int" compulsory 1.0 } }
"#;
    std::fs::write(temp_dir.join("container.txt"), container_txt).unwrap();

    let validators = load_validators(&temp_dir, None);
    let diagnostics = acs_text_diagnostics(&acs_text, &validators, None);
    assert!(
        diagnostics.is_empty(),
        "Expected no errors, found: {:?}",
        diagnostics
    );
    std::fs::remove_dir_all(&temp_dir).unwrap();
}

#[test]
fn test_filepath_and_scriptfileexists() {
    let temp_dir = std::env::temp_dir().join(format!(
        "acs_text_filepath_test_{:?}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    if temp_dir.exists() {
        std::fs::remove_dir_all(&temp_dir).unwrap();
    }
    std::fs::create_dir_all(&temp_dir).unwrap();

    let container_txt = r#"
test-container
{
  kind "container"
  top-level 1
  file { type "filepath" }
  file_edit { type "filepathedit" }
  script { type "string" validation "ScriptFileExists" }
}
"#;
    std::fs::write(temp_dir.join("container.txt"), container_txt).unwrap();

    // Create a dummy file for existence check
    std::fs::write(temp_dir.join("existing_file.txt"), "hello").unwrap();
    // Create a dummy script file with .gs
    std::fs::write(temp_dir.join("my_script.gs"), "print(\"hello\");").unwrap();

    let validators = load_validators(&temp_dir, None);

    // 1. Valid case
    let valid_content = r#"
test-container {
kind "test-container"
file "existing_file.txt"
file_edit "existing_file.txt"
script "my_script"
}
"#;
    let pairs = parse_acs_text(valid_content).unwrap();
    let acs_text = process_acs_text_ast(pairs, valid_content);
    let diagnostics = acs_text_diagnostics(
        &acs_text,
        &validators,
        Some(&temp_dir.join("test.acs_text")),
    );
    assert!(
        diagnostics.is_empty(),
        "Expected no diagnostics, found: {:?}",
        diagnostics
    );

    // 2. Invalid case
    let invalid_content = r#"
test-container {
kind "test-container"
file "non_existent.txt"
file_edit "non_existent.txt"
script "non_existent_script"
}
"#;
    let pairs_inv = parse_acs_text(invalid_content).unwrap();
    let acs_text_inv = process_acs_text_ast(pairs_inv, invalid_content);
    let diagnostics_inv = acs_text_diagnostics(
        &acs_text_inv,
        &validators,
        Some(&temp_dir.join("test_inv.acs_text")),
    );

    let has_file_error = diagnostics_inv.iter().any(|d| {
        d.message
            .contains("File 'non_existent.txt' for key 'file' does not exist")
    });
    let has_file_edit_error = diagnostics_inv.iter().any(|d| {
        d.message
            .contains("File 'non_existent.txt' for key 'file_edit' does not exist")
    });
    let has_script_error = diagnostics_inv.iter().any(|d| {
        d.message
            .contains("File 'non_existent_script' for key 'script' does not exist")
    });

    assert!(
        has_file_edit_error,
        "Expected error for non-existent file in file_edit"
    );
    assert!(
        has_script_error,
        "Expected error for non-existent file in script"
    );

    // This is the one that should fail currently (filepath missing existence check)
    assert!(
        has_file_error,
        "Expected error for non-existent file in filepath, but not found. Diagnostics: {:?}",
        diagnostics_inv
    );

    std::fs::remove_dir_all(&temp_dir).unwrap();
}

#[test]
fn test_trainzmesh_fbx_fallback() {
    let temp_dir = std::env::temp_dir().join(format!(
        "acs_text_trainzmesh_fbx_test_{:?}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    if temp_dir.exists() {
        std::fs::remove_dir_all(&temp_dir).unwrap();
    }
    std::fs::create_dir_all(&temp_dir).unwrap();

    let container_txt = r#"
test-container
{
  kind "container"
  top-level 1
  mesh { type "filepath" }
}
"#;
    std::fs::write(temp_dir.join("container.txt"), container_txt).unwrap();

    // Create a dummy fbx file
    std::fs::write(temp_dir.join("default.fbx"), "dummy fbx content").unwrap();

    let validators = load_validators(&temp_dir, None);

    // Test case: .trainzmesh is specified, but only .fbx exists
    let content = r#"
test-container {
kind "test-container"
mesh "default.trainzmesh"
}
"#;
    let pairs = parse_acs_text(content).unwrap();
    let acs_text = process_acs_text_ast(pairs, content);
    let diagnostics = acs_text_diagnostics(
        &acs_text,
        &validators,
        Some(&temp_dir.join("test.acs_text")),
    );

    // Currently this should fail (it should have one diagnostic about missing file)
    assert!(
        diagnostics.is_empty(),
        "Expected no diagnostics because of .fbx fallback, but found: {:?}",
        diagnostics
    );

    std::fs::remove_dir_all(&temp_dir).unwrap();
}

#[test]
fn test_filepath_table_fbx_fallback() {
    let temp_dir = std::env::temp_dir().join(format!(
        "acs_text_filepath_table_fbx_test_{:?}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    if temp_dir.exists() {
        std::fs::remove_dir_all(&temp_dir).unwrap();
    }
    std::fs::create_dir_all(&temp_dir).unwrap();

    let container_txt = r#"
test-container
{
  kind "container"
  top-level 1
  mesh-table { type "container" validation "FilepathTableFilesExist" }
}
"#;
    std::fs::write(temp_dir.join("container.txt"), container_txt).unwrap();

    // Create a dummy fbx file
    std::fs::write(temp_dir.join("default.fbx"), "dummy fbx content").unwrap();

    let validators = load_validators(&temp_dir, None);

    // Test case: .trainzmesh is specified in mesh-table, but only .fbx exists
    let content = r#"
test-container {
kind "test-container"
mesh-table {
  default { mesh "default.trainzmesh" }
}
}
"#;
    let pairs = parse_acs_text(content).unwrap();
    let acs_text = process_acs_text_ast(pairs, content);
    let _diagnostics = acs_text_diagnostics(
        &acs_text,
        &validators,
        Some(&temp_dir.join("test.acs_text")),
    );

    std::fs::remove_dir_all(&temp_dir).unwrap();
}

#[test]
fn test_array_element_sequential_validation() {
    let temp_dir = std::env::temp_dir().join(format!(
        "acs_text_array_seq_test_{:?}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&temp_dir).unwrap();

    let container_txt = r#"
my-container {
    kind "container"
    top-level 1
    array-element {
        container-type0 "type0"
        container-type1 "type1"
    }
}
type0 { kind "container" v0 { type "int" } }
type1 { kind "container" v1 { type "int" } }
"#;
    std::fs::write(temp_dir.join("container.txt"), container_txt).unwrap();
    let validators = load_validators(&temp_dir, None);

    // 1. Valid sequential
    let content1 = r#"
my-container {
    0 { v0 1 }
    1 { v1 2 }
}
"#;
    let acs_text1 = trainz_ast::acs_text::process::process_acs_text_ast(
        trainz_parser::acs_text::parse_acs_text(content1).unwrap(),
        content1,
    );
    let diagnostics1 = acs_text_diagnostics(&acs_text1, &validators, None);
    assert!(
        diagnostics1.is_empty(),
        "Expected no diagnostics, found: {:?}",
        diagnostics1
    );

    // 2. Non-sequential (gap)
    let content2 = r#"
my-container {
    0 { v0 1 }
    2 { v1 2 }
}
"#;
    let acs_text2 = trainz_ast::acs_text::process::process_acs_text_ast(
        trainz_parser::acs_text::parse_acs_text(content2).unwrap(),
        content2,
    );
    let diagnostics2 = acs_text_diagnostics(&acs_text2, &validators, None);
    assert!(
        diagnostics2
            .iter()
            .any(|d| d.message.contains("Non-sequential array index '2'")),
        "Expected warning for gap in indices"
    );
    assert!(
        diagnostics2
            .iter()
            .any(|d| d.message.contains("Missing tuple element '1'")),
        "Expected error for missing tuple element"
    );

    // 3. Out of bounds
    let content3 = r#"
my-container {
    0 { v0 1 }
    1 { v1 2 }
    2 { v0 3 }
}
"#;
    let acs_text3 = trainz_ast::acs_text::process::process_acs_text_ast(
        trainz_parser::acs_text::parse_acs_text(content3).unwrap(),
        content3,
    );
    let diagnostics3 = acs_text_diagnostics(&acs_text3, &validators, None);
    assert!(
        diagnostics3
            .iter()
            .any(|d| d.message.contains("Index '2' out of bounds")),
        "Expected error for out of bounds index"
    );

    std::fs::remove_dir_all(&temp_dir).unwrap();
}

#[test]
fn test_array_element_single_type_gap_validation() {
    let temp_dir = std::env::temp_dir().join(format!(
        "acs_text_array_single_test_{:?}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&temp_dir).unwrap();

    let container_txt = r#"
my-container {
    kind "container"
    top-level 1
    array-element {
        container-type0 "type0"
    }
}
type0 { kind "container" v0 { type "int" } }
"#;
    std::fs::write(temp_dir.join("container.txt"), container_txt).unwrap();
    let validators = load_validators(&temp_dir, None);

    // 1. Valid with gap
    let content1 = r#"
my-container {
    0 { v0 1 }
    1 { v0 2 }
    7 { v0 3 }
    8 { v0 4 }
}
"#;
    let acs_text1 = trainz_ast::acs_text::process::process_acs_text_ast(
        trainz_parser::acs_text::parse_acs_text(content1).unwrap(),
        content1,
    );
    let diagnostics1 = acs_text_diagnostics(&acs_text1, &validators, None);
    assert!(
        diagnostics1.is_empty(),
        "Expected no diagnostics for single type with gaps, found: {:?}",
        diagnostics1
    );

    // 2. Non-increasing
    let content2 = r#"
my-container {
    0 { v0 1 }
    1 { v0 2 }
    8 { v0 3 }
    7 { v0 4 }
}
"#;
    let acs_text2 = trainz_ast::acs_text::process::process_acs_text_ast(
        trainz_parser::acs_text::parse_acs_text(content2).unwrap(),
        content2,
    );
    let diagnostics2 = acs_text_diagnostics(&acs_text2, &validators, None);
    assert!(
        diagnostics2
            .iter()
            .any(|d| d.message.contains("Non-increasing array index '7'")),
        "Expected warning for non-increasing indices"
    );

    std::fs::remove_dir_all(&temp_dir).unwrap();
}

#[test]
fn test_tag_array_multi_type_validation() {
    let temp_dir = std::env::temp_dir().join(format!(
        "acs_text_tag_array_test_{:?}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&temp_dir).unwrap();

    let container_txt = r#"
my-container {
    kind "container"
    top-level 1
    tagarray {
        container-type0 "type0"
        container-type1 "type1"
    }
}
type0 { kind "container" v0 { type "int" } }
type1 { kind "container" v1 { type "int" } }
"#;
    std::fs::write(temp_dir.join("container.txt"), container_txt).unwrap();
    let validators = load_validators(&temp_dir, None);

    // 1. Valid order
    let content1 = r#"
my-container {
    foo { v0 1 }
    bar { v1 2 }
}
"#;
    let acs_text1 = trainz_ast::acs_text::process::process_acs_text_ast(
        trainz_parser::acs_text::parse_acs_text(content1).unwrap(),
        content1,
    );
    let diagnostics1 = acs_text_diagnostics(&acs_text1, &validators, None);
    assert!(
        diagnostics1.is_empty(),
        "Expected no diagnostics, found: {:?}",
        diagnostics1
    );

    // 2. Invalid types for order
    let content2 = r#"
my-container {
    foo { v1 1 }
    bar { v0 2 }
}
"#;
    let acs_text2 = trainz_ast::acs_text::process::process_acs_text_ast(
        trainz_parser::acs_text::parse_acs_text(content2).unwrap(),
        content2,
    );
    let diagnostics2 = acs_text_diagnostics(&acs_text2, &validators, None);
    assert!(
        diagnostics2
            .iter()
            .any(|d| d.message.contains("Unknown key 'v1' in container 'type0'")),
        "Expected error for v1 in type0 position"
    );
    assert!(
        diagnostics2
            .iter()
            .any(|d| d.message.contains("Unknown key 'v0' in container 'type1'")),
        "Expected error for v0 in type1 position"
    );

    // 3. Out of bounds for tagarray
    let content3 = r#"
my-container {
    foo { v0 1 }
    bar { v1 2 }
    baz { v0 3 }
}
"#;
    let acs_text3 = trainz_ast::acs_text::process::process_acs_text_ast(
        trainz_parser::acs_text::parse_acs_text(content3).unwrap(),
        content3,
    );
    let diagnostics3 = acs_text_diagnostics(&acs_text3, &validators, None);
    assert!(
        diagnostics3
            .iter()
            .any(|d| d.message.contains("Index '2' out of bounds")),
        "Expected error for too many items in tagarray"
    );

    std::fs::remove_dir_all(&temp_dir).unwrap();
}
