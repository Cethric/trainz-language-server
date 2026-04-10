use std::fs;
use tokio::time::{Duration, timeout};
use tower_lsp_server::ls_types::*;
use tower_lsp_server::{LanguageServer, LspService};
use trainz_language_server::state::GameScriptLanguageServer;

#[tokio::test]
async fn test_circular_include_deadlock() {
    let (service, _) = LspService::new(|client| {
        GameScriptLanguageServer::new(client, None, vec![], "test-version")
    });
    let temp_dir = std::env::current_dir()
        .unwrap()
        .join("target/repro_deadlock_test");
    fs::create_dir_all(&temp_dir).unwrap();

    let path_a = temp_dir.join("A.gs");
    let path_b = temp_dir.join("B.gs");

    // Circular include: A.gs -> B.gs -> A.gs
    fs::write(&path_a, "include \"B.gs\"\nclass A {};").unwrap();
    fs::write(&path_b, "include \"A.gs\"\nclass B {};").unwrap();

    let uri_a = Uri::from_file_path(&path_a).unwrap();

    let did_open_params = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri_a.clone(),
            language_id: "game-script".to_string(),
            version: 1,
            text: fs::read_to_string(&path_a).unwrap(),
        },
    };

    // This should complete without deadlock or stack overflow
    let result = timeout(
        Duration::from_secs(5),
        service.inner().did_open(did_open_params),
    )
    .await;

    assert!(
        result.is_ok(),
        "did_open timed out - likely a deadlock or infinite recursion!"
    );

    // Clean up
    let _ = fs::remove_dir_all(&temp_dir);
}
