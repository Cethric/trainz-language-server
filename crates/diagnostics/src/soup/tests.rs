use super::validator::*;
use tower_lsp_server::ls_types::DiagnosticSeverity;
use trainz_ast::soup::process::process_soup_ast;
use trainz_parser::soup::parse_soup;
use trainz_soup_validators::{load_validators, ContainerValidator, Validators};

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
    let pairs = parse_soup(content).unwrap();
    let soup = process_soup_ast(pairs, content);

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

    let validators = load_validators(&temp_dir);
    let diagnostics = soup_diagnostics(&soup, &validators, None);

    let errors: Vec<_> = diagnostics
        .iter()
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
fn test_case_insensitive_keys_in_soup() {
    let content = r#"
Signals {
0 { light -1 }
1 { light -1 }
2 { light -1 }
3 { light -1 }
}
"#;
    let pairs = parse_soup(content);
    assert!(pairs.is_ok());

    let pairs = pairs.unwrap();
    let soup = process_soup_ast(pairs, content);

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

    let validators = load_validators(&temp_dir);
    let diagnostics = soup_diagnostics(&soup, &validators, None);

    // Check if there are any errors.
    // 1. "Signals" should match "signals" in container.txt
    // 2. "Light" should match "light" in signal-possibility
    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.severity == Some(DiagnosticSeverity::ERROR))
        .collect();
    let warnings: Vec<_> = diagnostics
        .iter()
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
    let pairs2 = parse_soup(content2).unwrap();
    let soup2 = process_soup_ast(pairs2, content2);

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

    let validators = load_validators(&temp_dir);
    let diagnostics = soup_diagnostics(&soup2, &validators, None);
    let has_duplicate_error = diagnostics.iter().any(|diag| {
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
fn test_kind_txt_validation() {
    let content = r#"
kind "my-kind"
key1 "value1"
key2 123
"#;
    let pairs = parse_soup(content).unwrap();
    let soup = process_soup_ast(pairs, content);

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

    let validators = load_validators(&temp_dir);
    let diagnostics = soup_diagnostics(&soup, &validators, None);

    let errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.severity == Some(DiagnosticSeverity::ERROR))
        .collect();
    assert!(
        errors.is_empty(),
        "Expected no errors for kind-based validation, but found: {:?}",
        errors
    );

    // Test with invalid value
    let invalid_content = r#"
kind "my-kind"
key1 123
"#;
    let pairs_invalid = parse_soup(invalid_content).unwrap();
    let soup_invalid = process_soup_ast(pairs_invalid, invalid_content);
    let diagnostics_invalid = soup_diagnostics(&soup_invalid, &validators, None);
    eprintln!(
        "DEBUG: Diagnostics for invalid content: {:?}",
        diagnostics_invalid
    );
    let has_type_error = diagnostics_invalid
        .iter()
        .any(|d| d.message.contains("Invalid type for key 'key1'"));
    assert!(
        has_type_error,
        "Expected type error for key1, but found: {:?}",
        diagnostics_invalid
    );

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_numeric_keys_in_soup() {
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
    let pairs = parse_soup(content);
    assert!(
        pairs.is_ok(),
        "Failed to parse soup with numeric keys: {:?}",
        pairs.err()
    );

    let pairs = pairs.unwrap();
    let soup = process_soup_ast(pairs, content);

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

    let validators = load_validators(&temp_dir);
    let diagnostics = soup_diagnostics(&soup, &validators, None);

    // Check if there are any errors. Numeric keys should be valid.
    let errors: Vec<_> = diagnostics
        .iter()
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
    let pairs = parse_soup(content);
    assert!(pairs.is_ok());

    let pairs = pairs.unwrap();
    let soup = process_soup_ast(pairs, content);

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

    let validators = load_validators(&temp_dir);
    let diagnostics = soup_diagnostics(&soup, &validators, None);

    // Check for duplicate key error.
    let has_duplicate_error = diagnostics.iter().any(|diag| {
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
    let pairs = parse_soup(content);
    assert!(pairs.is_ok());

    let pairs = pairs.unwrap();
    let soup = process_soup_ast(pairs, content);

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

    let validators = load_validators(&temp_dir);
    let diagnostics = soup_diagnostics(&soup, &validators, None);

    let errors: Vec<_> = diagnostics
        .iter()
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
    let pairs = parse_soup(content).unwrap();
    let soup = process_soup_ast(pairs, content);

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

    let validators = load_validators(&temp_dir);
    let diagnostics = soup_diagnostics(&soup, &validators, None);

    let unknown_key_errors: Vec<_> = diagnostics
        .iter()
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
fn test_compulsory_and_obsolete_validation() {
    let content = r#"
    kind "test-container"
    required-key "present"
    obsolete-key "should-warn"
    "#;
    let pairs = parse_soup(content).unwrap();
    let soup = process_soup_ast(pairs, content);

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

    let validators = load_validators(&temp_dir);
    let diagnostics = soup_diagnostics(&soup, &validators, None);

    // Should have no errors for missing optional keys
    let missing_errors: Vec<_> = diagnostics
        .iter()
        .filter(|diag| {
            diag.message.contains("is missing") && diag.severity == Some(DiagnosticSeverity::ERROR)
        })
        .collect();

    assert!(
        missing_errors.is_empty(),
        "Expected no missing key errors for optional keys, but found: {:?}",
        missing_errors
    );

    // Should have a warning for obsolete-key
    let obsolete_warning = diagnostics.iter().find(|diag| {
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
    let pairs = parse_soup(content).unwrap();
    let soup = process_soup_ast(pairs, content);

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

    let validators = load_validators(&temp_dir);
    let diagnostics = soup_diagnostics(&soup, &validators, None);

    // Should have an error for missing required-key
    let missing_error = diagnostics.iter().find(|diag| {
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
    let pairs = parse_soup(content).unwrap();
    let soup = process_soup_ast(pairs, content);

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

    let validators = load_validators(&temp_dir);
    let diagnostics = soup_diagnostics(&soup, &validators, None);

    let has_type_error = diagnostics
        .iter()
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
    let pairs = parse_soup(content).unwrap();
    let soup = process_soup_ast(pairs, content);

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

    let validators = load_validators(&temp_dir);
    let diagnostics = soup_diagnostics(&soup, &validators, None);

    let has_type_error = diagnostics
        .iter()
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
    let pairs = parse_soup(content).expect("Failed to parse soup");
    let soup = process_soup_ast(pairs, content);

    // Mock validator that has a "string-table" container but NOT marked as TagArray
    let mut validators = Validators::default();
    validators.containers.push(ContainerValidator {
        container_name: "string-table".to_string(),
        top_level: true, // Let's say it's top-level
        allow_any_key: true,
        ..Default::default()
    });

    let diagnostics = soup_diagnostics(&soup, &validators, None);

    // We expect NO "Unknown key" errors for 'description', etc., according to the requirement.
    for diag in &diagnostics {
        println!("Diagnostic: {}", diag.message);
    }

    let has_unknown_key_error = diagnostics
        .iter()
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
    let pairs = parse_soup(content).unwrap();
    let soup = process_soup_ast(pairs, content);

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

    let validators = load_validators(&temp_dir);
    let diagnostics = soup_diagnostics(&soup, &validators, None);

    // 1. Check for duplicate key error for "Key1"
    let has_duplicate_error = diagnostics
        .iter()
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
    let pairs_invalid = parse_soup(content_invalid).unwrap();
    let soup_invalid = process_soup_ast(pairs_invalid, content_invalid);
    let validators = load_validators(&temp_dir);
    let diagnostics_invalid = soup_diagnostics(&soup_invalid, &validators, None);

    let has_unknown_key_error = diagnostics_invalid
        .iter()
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
    let pairs = parse_soup(content).unwrap();
    let soup = process_soup_ast(pairs, content);
    let validators = load_validators(&temp_dir);
    let diagnostics = soup_diagnostics(&soup, &validators, None);

    let has_error = diagnostics
        .iter()
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
    let validators_mixed = load_validators(&temp_dir);
    let diagnostics_mixed = soup_diagnostics(&soup, &validators_mixed, None);
    let has_error_mixed = diagnostics_mixed
        .iter()
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
    let validators_tagarray = load_validators(&temp_dir);
    let diagnostics_tagarray = soup_diagnostics(&soup, &validators_tagarray, None);
    let has_error_tagarray = diagnostics_tagarray
        .iter()
        .any(|d| d.severity == Some(DiagnosticSeverity::ERROR));
    assert!(
        !has_error_tagarray,
        "Unexpected errors with tag-array: {:?}",
        diagnostics_tagarray
    );

    // Check that tagarray itself is ignored in soup
    let content_meta = r#"
string-table {
key "value"
tagarray { }
tag-array { }
}
"#;
    let pairs_meta = parse_soup(content_meta).unwrap();
    let soup_meta = process_soup_ast(pairs_meta, content_meta);
    let diagnostics_meta = soup_diagnostics(&soup_meta, &validators, None);
    let has_unknown_key = diagnostics_meta
        .iter()
        .any(|d| d.message.contains("Unknown key"));
    assert!(
        !has_unknown_key,
        "tagarray/tag-array should be ignored as metadata keys in Soup: {:?}",
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
    let pairs = parse_soup(content).unwrap();
    let soup = process_soup_ast(pairs, content);

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

    let validators = load_validators(&temp_dir);
    let diagnostics = soup_diagnostics(&soup, &validators, None);
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
    let pairs_missing = parse_soup(content_missing).unwrap();
    let soup_missing = process_soup_ast(pairs_missing, content_missing);
    let diagnostics_missing = soup_diagnostics(&soup_missing, &validators, None);

    let has_missing_key_error = diagnostics_missing.iter().any(|diag| {
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
fn test_array_element_sequential_validation() {
    let content = r#"
my-array {
kind "my-array"
0 {
    value "elem0"
}
1 {
    value "elem1"
}
}
"#;
    let pairs = parse_soup(content).unwrap();
    let soup = process_soup_ast(pairs, content);

    let temp_dir = std::env::current_dir()
        .unwrap()
        .join("temp_array_element_test");
    if temp_dir.exists() {
        std::fs::remove_dir_all(&temp_dir).unwrap();
    }
    std::fs::create_dir_all(&temp_dir).unwrap();

    let container_txt = r#"
my-array
{
  kind "container"
  array-element
  {
container-type0 "my-element"
  }
}

my-element
{
  kind "container"
  value
  {
type "string"
  }
}
"#;
    std::fs::write(temp_dir.join("container.txt"), container_txt).unwrap();

    let validators = load_validators(&temp_dir);
    let diagnostics = soup_diagnostics(&soup, &validators, None);
    assert!(
        diagnostics.is_empty(),
        "Expected no errors for valid sequential array, but found: {:?}",
        diagnostics
    );

    // Non-sequential keys
    let content_non_seq = r#"
my-array {
kind "my-array"
0 {
    value "elem0"
}
2 {
    value "elem2"
}
}
"#;
    let pairs_non_seq = parse_soup(content_non_seq).unwrap();
    let soup_non_seq = process_soup_ast(pairs_non_seq, content_non_seq);
    let diagnostics_non_seq = soup_diagnostics(&soup_non_seq, &validators, None);

    let has_seq_error = diagnostics_non_seq.iter().any(|diag| {
        diag.message.contains("Non-sequential array index '2'")
            && diag.message.contains("Expected '1'")
    });
    assert!(
        has_seq_error,
        "Expected non-sequential error, but found: {:?}",
        diagnostics_non_seq
    );

    std::fs::remove_dir_all(&temp_dir).unwrap();
}

#[test]
fn test_top_level_inheritance() {
    let temp_dir = std::env::temp_dir().join("trainz-lsp-test-top-level");
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

    let soup_content = r#"
kind "obsolete-track"
asset-id "test-asset"
track-id "test-track"
"#;
    let pairs = parse_soup(soup_content).unwrap();
    let soup = process_soup_ast(pairs, soup_content);

    let validators = load_validators(&temp_dir);
    let diagnostics = soup_diagnostics(&soup, &validators, None);

    // Should NOT have any "Unknown key" errors if top_level validation worked
    let has_unknown_key = diagnostics
        .iter()
        .any(|d| d.message.contains("Unknown key"));
    assert!(
        !has_unknown_key,
        "Unexpected unknown key errors: {:?}",
        diagnostics
    );

    // Should NOT have error for 'asset-id' as it is inherited from base-asset
    let has_missing_asset_id = diagnostics
        .iter()
        .any(|d| d.message.contains("Compulsory key 'asset-id' is missing"));
    assert!(
        !has_missing_asset_id,
        "Unexpected error for 'asset-id': {:?}",
        diagnostics
    );

    // Should NOT have error for 'track-id' as it is inherited from itrack
    let has_missing_track_id = diagnostics
        .iter()
        .any(|d| d.message.contains("Compulsory key 'track-id' is missing"));
    assert!(
        !has_missing_track_id,
        "Unexpected error for 'track-id': {:?}",
        diagnostics
    );

    // Now test missing compulsory kind
    let soup_content_missing = r#"
kind "obsolete-track"
"#;
    let pairs_missing = parse_soup(soup_content_missing).unwrap();
    let soup_missing = process_soup_ast(pairs_missing, soup_content_missing);
    let diagnostics_missing = soup_diagnostics(&soup_missing, &validators, None);

    let has_missing_asset_id = diagnostics_missing
        .iter()
        .any(|d| d.message.contains("Compulsory key 'asset-id' is missing"));
    assert!(
        has_missing_asset_id,
        "Expected error for missing 'asset-id', but got: {:?}",
        diagnostics_missing
    );

    let has_missing_track_id = diagnostics_missing
        .iter()
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
    let temp_dir = std::env::temp_dir().join("trainz-lsp-test-subpossibilities-case");
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

    let validators = load_validators(&temp_dir);

    // 1. Test case-insensitive SubPossibilities in validator definition
    let soup_content = r#"
test-container
{
  mandatory-key "value"
}
"#;
    let pairs = parse_soup(soup_content).unwrap();
    let soup = process_soup_ast(pairs, soup_content);
    let diagnostics = soup_diagnostics(&soup, &validators, None);

    let has_error = diagnostics
        .iter()
        .any(|d| d.severity == Some(DiagnosticSeverity::ERROR));
    assert!(
        !has_error,
        "Unexpected errors with SubPossibilities: {:?}",
        diagnostics
    );

    // 2. Test missing compulsory key defined in SubPossibilities
    let soup_content_missing = r#"
test-container
{
  Optional-Key "value"
}
"#;
    let pairs_missing = parse_soup(soup_content_missing).unwrap();
    let soup_missing = process_soup_ast(pairs_missing, soup_content_missing);
    let diagnostics_missing = soup_diagnostics(&soup_missing, &validators, None);

    let has_missing_key = diagnostics_missing.iter().any(|d| {
        d.message
            .contains("Compulsory key 'mandatory-key' is missing")
    });
    assert!(
        has_missing_key,
        "Expected error for missing 'mandatory-key' defined in SubPossibilities: {:?}",
        diagnostics_missing
    );

    // 3. Test that SubPossibilities as a key in soup itself is ignored (not flagged as unknown)
    let soup_with_metadata = r#"
test-container
{
  mandatory-key "value"
  SubPossibilities { }
  subpossibilities { }
}
"#;
    let pairs_metadata = parse_soup(soup_with_metadata).unwrap();
    let soup_metadata = process_soup_ast(pairs_metadata, soup_with_metadata);
    let diagnostics_metadata = soup_diagnostics(&soup_metadata, &validators, None);

    let has_unknown_key = diagnostics_metadata
        .iter()
        .any(|d| d.message.contains("Unknown key"));
    assert!(
        !has_unknown_key,
        "SubPossibilities should be ignored as a metadata key in Soup: {:?}",
        diagnostics_metadata
    );

    std::fs::remove_dir_all(&temp_dir).unwrap();
}

#[test]
fn test_category_class_validation() {
    let temp_dir = std::env::temp_dir().join("trainz-lsp-test-cat-class");
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
category-class {
    type string
    validation IsValidCategoryClass
}
}
"#;
    std::fs::write(temp_dir.join("container.txt"), container_content).unwrap();

    let soup_content = r#"
my_container {
kind "my_container"
category-class "Scenery"
}
"#;
    let pairs = parse_soup(soup_content).unwrap();
    let soup = process_soup_ast(pairs, soup_content);

    let validators = load_validators(&temp_dir);
    let diagnostics = soup_diagnostics(&soup, &validators, None);

    assert!(
        diagnostics.is_empty(),
        "Unexpected diagnostics for valid category class: {:?}",
        diagnostics
    );

    let soup_content_invalid = r#"
my_container {
category-class "InvalidClass"
}
"#;
    let pairs_invalid = parse_soup(soup_content_invalid).unwrap();
    let soup_invalid = process_soup_ast(pairs_invalid, soup_content_invalid);
    let diagnostics_invalid = soup_diagnostics(&soup_invalid, &validators, None);
    println!("Diagnostics invalid: {:?}", diagnostics_invalid);

    let has_cat_class_error = diagnostics_invalid
        .iter()
        .any(|d| d.message.contains("is not a valid category class"));
    assert!(
        has_cat_class_error,
        "Expected category class error, but got: {:?}",
        diagnostics_invalid
    );

    // Test with "WAT" as requested by user
    let soup_content_wat = r#"
my_container {
category-class "WAT"
}
"#;
    let pairs_wat = parse_soup(soup_content_wat).unwrap();
    let soup_wat = process_soup_ast(pairs_wat, soup_content_wat);
    let diagnostics_wat = soup_diagnostics(&soup_wat, &validators, None);

    let has_wat_error = diagnostics_wat.iter().any(|d| {
        d.message
            .contains("Value 'WAT' for key 'category-class' is not a valid category class")
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

    let soup_content_region = r#"
region_container {
category-region "FRA"
}
"#;
    let pairs_region = parse_soup(soup_content_region).unwrap();
    let soup_region = process_soup_ast(pairs_region, soup_content_region);
    let validators_region = load_validators(&temp_dir);
    let diagnostics_region = soup_diagnostics(&soup_region, &validators_region, None);
    assert!(
        diagnostics_region.is_empty(),
        "Unexpected diagnostics for valid category region: {:?}",
        diagnostics_region
    );

    let soup_content_fra_invalid = r#"
region_container {
category-region "GER"
}
"#;
    let pairs_fra_invalid = parse_soup(soup_content_fra_invalid).unwrap();
    let soup_fra_invalid = process_soup_ast(pairs_fra_invalid, soup_content_fra_invalid);
    let diagnostics_fra_invalid = soup_diagnostics(&soup_fra_invalid, &validators_region, None);
    let has_region_error = diagnostics_fra_invalid
        .iter()
        .any(|d| d.message.contains("is not a valid category region"));
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
category-era {
    type string
    validation IsValidCategoryEra
}
}
"#;
    std::fs::write(temp_dir.join("container.txt"), container_content_era).unwrap();

    let soup_content_era = r#"
era_container {
category-era "2000s;2010s;"
}
"#;
    let pairs_era = parse_soup(soup_content_era).unwrap();
    let soup_era = process_soup_ast(pairs_era, soup_content_era);
    let validators_era = load_validators(&temp_dir);
    let diagnostics_era = soup_diagnostics(&soup_era, &validators_era, None);
    assert!(
        diagnostics_era.is_empty(),
        "Unexpected diagnostics for valid category era: {:?}",
        diagnostics_era
    );

    let soup_content_era_invalid = r#"
era_container {
category-era "2000s;2030s;2010s;"
}
"#;
    let pairs_era_invalid = parse_soup(soup_content_era_invalid).unwrap();
    let soup_era_invalid = process_soup_ast(pairs_era_invalid, soup_content_era_invalid);
    let diagnostics_era_invalid = soup_diagnostics(&soup_era_invalid, &validators_era, None);
    let has_era_error = diagnostics_era_invalid.iter().any(|d| {
        d.message
            .contains("Value '2030s' for key 'category-era' is not a valid category era.")
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
    let temp_dir = std::env::temp_dir().join("trainz-lsp-test-disabled");
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

    let validators = load_validators(&temp_dir);

    // 1. Test that disabled key does not trigger missing compulsory error (as child)
    let soup_content = r#"
test-container {
  normal-key "value"
}
"#;
    let pairs = parse_soup(soup_content).unwrap();
    let soup = process_soup_ast(pairs, soup_content);
    let diagnostics = soup_diagnostics(&soup, &validators, None);
    assert!(
        diagnostics.is_empty(),
        "Unexpected diagnostics for missing disabled key: {:?}",
        diagnostics
    );

    // 2. Test that disabled key does not trigger warning when present
    let soup_content_present = r#"
test-container {
  normal-key "value"
  disabled-key "whatever"
}
"#;
    let pairs_present = parse_soup(soup_content_present).unwrap();
    let soup_present = process_soup_ast(pairs_present, soup_content_present);
    let diagnostics_present = soup_diagnostics(&soup_present, &validators, None);
    assert!(
        diagnostics_present.is_empty(),
        "Unexpected diagnostics for present disabled key: {:?}",
        diagnostics_present
    );

    // 3. Test top-level disabled key
    let kind_content_top = r#"
kind "test-kind"
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
"#;
    std::fs::write(temp_dir.join("kind.txt"), kind_content_top).unwrap();
    let validators_top = load_validators(&temp_dir);

    let soup_top = r#"
kind "test-kind"
normal-top "value"
"#;
    let pairs_top = parse_soup(soup_top).unwrap();
    let soup_obj_top = process_soup_ast(pairs_top, soup_top);
    let diagnostics_top = soup_diagnostics(&soup_obj_top, &validators_top, None);
    assert!(
        diagnostics_top.is_empty(),
        "Unexpected diagnostics for missing disabled top-level key: {:?}",
        diagnostics_top
    );

    std::fs::remove_dir_all(&temp_dir).unwrap();
}

#[test]
fn test_type_combobox_listbox_filepathedit() {
    let temp_dir = std::env::temp_dir().join("soup_type_test");
    if temp_dir.exists() {
        std::fs::remove_dir_all(&temp_dir).unwrap();
    }
    std::fs::create_dir_all(&temp_dir).unwrap();

    let container_txt = r#"
test-container
{
  combo { type "combobox" }
  list { type "listbox" }
  file { type "filepathedit" }
}
"#;
    std::fs::write(temp_dir.join("container.txt"), container_txt).unwrap();

    // Create a dummy file for filepathedit test
    std::fs::write(temp_dir.join("existing_file.txt"), "hello").unwrap();

    let validators = load_validators(&temp_dir);

    // 1. Valid cases
    let valid_content = r#"
test-container {
combo "single_value"
list "val1;val2;val3;"
file "existing_file.txt"
}
"#;
    let pairs = parse_soup(valid_content).unwrap();
    let soup = process_soup_ast(pairs, valid_content);
    let diagnostics = soup_diagnostics(&soup, &validators, Some(&temp_dir.join("test.soup")));
    assert!(
        diagnostics.is_empty(),
        "Expected no diagnostics for valid types, but found: {:?}",
        diagnostics
    );

    // 2. Invalid cases
    let invalid_content = r#"
test-container {
combo "val1;val2"
file "non_existent.txt"
}
"#;
    let pairs_inv = parse_soup(invalid_content).unwrap();
    let soup_inv = process_soup_ast(pairs_inv, invalid_content);
    let diagnostics_inv = soup_diagnostics(
        &soup_inv,
        &validators,
        Some(&temp_dir.join("test_inv.soup")),
    );

    let has_combo_warning = diagnostics_inv.iter().any(|d| {
        d.severity == Some(DiagnosticSeverity::WARNING)
            && d.message.contains("expects a single string value")
    });
    let has_file_error = diagnostics_inv.iter().any(|d| {
        d.severity == Some(DiagnosticSeverity::ERROR) && d.message.contains("does not exist")
    });

    assert!(
        has_combo_warning,
        "Expected warning for combobox with semicolon, but found: {:?}",
        diagnostics_inv
    );
    assert!(
        has_file_error,
        "Expected error for non-existent file in filepathedit, but found: {:?}",
        diagnostics_inv
    );

    std::fs::remove_dir_all(&temp_dir).unwrap();
}

#[test]
fn test_array_floatlist_validation() {
    let temp_dir = std::env::temp_dir().join("soup_floatlist_test");
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

    let validators = load_validators(&temp_dir);

    // 1. Array with only integers - should be valid for floatlist
    let int_content = r#"
test-container {
floats 1,2,3
}
"#;
    let pairs = parse_soup(int_content).unwrap();
    let soup = process_soup_ast(pairs, int_content);
    let diagnostics = soup_diagnostics(&soup, &validators, None);
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
    let pairs_f = parse_soup(float_content).unwrap();
    let soup_f = process_soup_ast(pairs_f, float_content);
    let diagnostics_f = soup_diagnostics(&soup_f, &validators, None);

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
    let pairs_i = parse_soup(invalid_content).unwrap();
    let soup_i = process_soup_ast(pairs_i, invalid_content);
    let diagnostics_i = soup_diagnostics(&soup_i, &validators, None);
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
    let validators_2 = load_validators(&temp_dir);
    let float_content_2 = r#"
test-container-2 {
arr 1.5,2.5
}
"#;
    let pairs_f2 = parse_soup(float_content_2).unwrap();
    let soup_f2 = process_soup_ast(pairs_f2, float_content_2);
    let diagnostics_f2 = soup_diagnostics(&soup_f2, &validators_2, None);
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
    let pairs = parse_soup(content).unwrap();
    let soup = process_soup_ast(pairs, content);

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

    let validators = load_validators(&temp_dir);
    let diagnostics = soup_diagnostics(&soup, &validators, None);
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
    let pairs_missing = parse_soup(content_missing).unwrap();
    let soup_missing = process_soup_ast(pairs_missing, content_missing);
    let diagnostics_missing = soup_diagnostics(&soup_missing, &validators, None);
    assert!(
        diagnostics_missing
            .iter()
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
    let pairs_extra = parse_soup(content_extra).unwrap();
    let soup_extra = process_soup_ast(pairs_extra, content_extra);
    let diagnostics_extra = soup_diagnostics(&soup_extra, &validators, None);
    assert!(
        diagnostics_extra
            .iter()
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
    let pairs_wrong_type = parse_soup(content_wrong_type).unwrap();
    let soup_wrong_type = process_soup_ast(pairs_wrong_type, content_wrong_type);
    let diagnostics_wrong_type = soup_diagnostics(&soup_wrong_type, &validators, None);
    // Should have an error about missing compulsory key 'value_str' in 'string-elem'
    assert!(
        diagnostics_wrong_type
            .iter()
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
    let pairs = parse_soup(content).unwrap();
    let soup = process_soup_ast(pairs, content);

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

    let validators = load_validators(&temp_dir);
    let diagnostics = soup_diagnostics(&soup, &validators, None);
    assert!(
        diagnostics.is_empty(),
        "Expected no errors, found: {:?}",
        diagnostics
    );
    std::fs::remove_dir_all(&temp_dir).unwrap();
}
