use crate::load_validators;
use std::path::PathBuf;
use trainz_ast::Range;
use trainz_ast::acs_text::KeyValuePair;

fn create_path(keys: &[&str]) -> Vec<KeyValuePair> {
    keys.iter()
        .map(|k| KeyValuePair {
            key: k.to_string(),
            key_range: Range::default(),
            value: None,
            range: Range::default(),
        })
        .collect()
}

fn to_refs(path: &Vec<KeyValuePair>) -> Vec<&KeyValuePair> {
    path.iter().collect()
}

#[tokio::test]
async fn test_load_validators() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data");
    let result = load_validators::load_validators(&path, None).await;
    assert!(
        result.is_ok(),
        "Failed to load validators: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn test_traversal_number() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data");
    let mut graph = load_validators::load_validators(&path, None)
        .await
        .expect("Failed to load");
    graph.update_inheritance();
    let path = create_path(&["thumbnails", "0"]);
    let node = graph.get_node_by_path(None, &to_refs(&path));
    assert!(node.is_some(), "Node not found");
    assert_eq!(node.unwrap().name(), "thumbnails-element");
}
#[tokio::test]
async fn test_traversal_alpha() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data");
    let mut graph = load_validators::load_validators(&path, None)
        .await
        .expect("Failed to load");
    graph.update_inheritance();
    let path = create_path(&["thumbnails", "name"]);
    let node = graph.get_node_by_path(None, &to_refs(&path));
    assert!(node.is_some(), "Node not found");
    assert_eq!(node.unwrap().name(), "thumbnails-element");
}

#[tokio::test]
async fn test_traversal_nested_number() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data");
    let mut graph = load_validators::load_validators(&path, None)
        .await
        .expect("Failed to load");
    graph.update_inheritance();
    let path = create_path(&["thumbnails", "0", "example"]);
    let node = graph.get_node_by_path(None, &to_refs(&path));
    assert!(node.is_some(), "Node not found");
    assert_eq!(node.unwrap().name(), "example");
}

#[tokio::test]
async fn test_traversal_nested_alpha() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data");
    let mut graph = load_validators::load_validators(&path, None)
        .await
        .expect("Failed to load");
    graph.update_inheritance();
    let path = create_path(&["thumbnails", "alpha", "example"]);
    let node = graph.get_node_by_path(None, &to_refs(&path));
    assert!(node.is_some(), "Node not found");
    assert_eq!(node.unwrap().name(), "example");
}

#[tokio::test]
async fn test_traversal_multiple_containers() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data");
    let mut graph = load_validators::load_validators(&path, None)
        .await
        .expect("Failed to load");
    graph.update_inheritance();

    // Test discriminator based lookup
    // kind-value-A -> thumbnails-element
    let path_a = create_path(&["thumbnails", "kind-value-A"]);
    let node_a = graph.get_node_by_path(None, &to_refs(&path_a));
    assert!(node_a.is_some(), "Node kind-value-A not found");
    assert_eq!(node_a.unwrap().name(), "thumbnails-element");

    // kind-value-B -> conditions
    let path_b = create_path(&["thumbnails", "kind-value-B"]);
    let node_b = graph.get_node_by_path(None, &to_refs(&path_b));
    assert!(node_b.is_some(), "Node kind-value-B not found");
    assert_eq!(node_b.unwrap().name(), "conditions");
}

#[tokio::test]
async fn test_traversal_empty_path() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data");
    let mut graph = load_validators::load_validators(&path, None)
        .await
        .expect("Failed to load");
    graph.update_inheritance();
    let path: Vec<KeyValuePair> = vec![];
    let node = graph.get_node_by_path(None, &to_refs(&path));
    assert!(node.is_none(), "Node should be none for empty path");
}

#[tokio::test]
async fn test_traversal_key_based_lookup() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data");
    let mut graph = load_validators::load_validators(&path, None)
        .await
        .expect("Failed to load");
    graph.update_inheritance();

    // Test key based lookup
    // effects -> name-id -> name-id-element
    let path_name_id = create_path(&["effects", "name-id"]);
    let node_name_id = graph.get_node_by_path(None, &to_refs(&path_name_id));
    assert!(node_name_id.is_some(), "Node name-id not found");
    assert_eq!(node_name_id.unwrap().name(), "name-id-element");

    // Test deep traversal: effects -> name-id -> kind
    let path_deep = create_path(&["effects", "name-id", "kind"]);
    let node_deep = graph.get_node_by_path(None, &to_refs(&path_deep));
    assert!(node_deep.is_some(), "Node kind not found in deep traversal");
    assert_eq!(node_deep.unwrap().name(), "kind");

    // helper -> helper-element
    let path_helper = create_path(&["effects", "helper"]);
    let node_helper = graph.get_node_by_path(None, &to_refs(&path_helper));
    assert!(node_helper.is_some(), "Node helper not found");
    assert_eq!(node_helper.unwrap().name(), "helper-element");
}
