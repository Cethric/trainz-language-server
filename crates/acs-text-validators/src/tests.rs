use crate::load_validators;
use std::path::PathBuf;

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
