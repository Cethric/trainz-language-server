use rayon::prelude::*;
use std::fs;
use tower_lsp_server::ls_types::*;
use tower_lsp_server::{LanguageServer, LspService};
use trainz_language_server::state::GameScriptLanguageServer;

#[tokio::test]
async fn test_acs_text_document_symbols() {
    let (service, _) = LspService::new(|client| {
        GameScriptLanguageServer::new(client, None, vec![], "test-version")
    });

    let temp_dir = std::env::current_dir()
        .unwrap()
        .join("target")
        .join("test_acs_text_document_symbols");
    if temp_dir.exists() {
        fs::remove_dir_all(&temp_dir).unwrap();
    }
    fs::create_dir_all(&temp_dir).unwrap();

    let config_path = temp_dir.join("config.txt");
    let config_content = r#"kuid <kuid:1:1>
kind "asset"
description "Test Asset"

thumbnails
{
  0
  {
    image "thumb.jpg"
    width 240
    height 180
  }
}"#;
    fs::write(&config_path, config_content).unwrap();

    let config_uri = Uri::from_file_path(&config_path).unwrap();

    // Initialize server with workspace folder
    service
        .inner()
        .initialize(InitializeParams {
            workspace_folders: Some(vec![WorkspaceFolder {
                uri: Uri::from_file_path(&temp_dir).unwrap(),
                name: "test".to_string(),
            }]),
            ..Default::default()
        })
        .await
        .unwrap();

    // Open config.txt
    service
        .inner()
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: config_uri.clone(),
                language_id: "acs_text".to_string(),
                version: 1,
                text: config_content.to_string(),
            },
        })
        .await;

    let params = DocumentSymbolParams {
        text_document: TextDocumentIdentifier {
            uri: config_uri.clone(),
        },
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
    };

    let result = service.inner().document_symbol(params).await.unwrap();

    if let Some(DocumentSymbolResponse::Nested(symbols)) = result {
        // Find kuid symbol
        let kuid_symbol = symbols
            .par_iter()
            .find_first(|s| s.name == "kuid" && s.kind == SymbolKind::PROPERTY);
        assert!(
            kuid_symbol.is_some(),
            "Should find 'kuid' symbol. Symbols: {:?}",
            symbols
        );

        // Find kind symbol
        let kind_symbol = symbols
            .par_iter()
            .find_first(|s| s.name == "kind" && s.kind == SymbolKind::PROPERTY);
        assert!(kind_symbol.is_some(), "Should find 'kind' symbol");

        // Find thumbnails symbol
        let thumbnails_symbol = symbols
            .par_iter()
            .find_first(|s| s.name == "thumbnails" && s.kind == SymbolKind::NAMESPACE);
        assert!(
            thumbnails_symbol.is_some(),
            "Should find 'thumbnails' symbol"
        );

        // Find nested thumbnails children
        let thumb_0_symbol = thumbnails_symbol
            .unwrap()
            .children
            .as_ref()
            .unwrap()
            .par_iter()
            .find_first(|s| s.name == "0" && s.kind == SymbolKind::NAMESPACE);
        assert!(
            thumb_0_symbol.is_some(),
            "Should find '0' symbol in thumbnails"
        );

        let image_symbol = thumb_0_symbol
            .unwrap()
            .children
            .as_ref()
            .unwrap()
            .par_iter()
            .find_first(|s| s.name == "image" && s.kind == SymbolKind::PROPERTY);
        assert!(
            image_symbol.is_some(),
            "Should find 'image' symbol in thumbnails/0"
        );
    } else {
        panic!("Expected nested document symbols, got {:?}", result);
    }

    let _ = fs::remove_dir_all(&temp_dir);
}
