use std::fs;
use tower_lsp_server::ls_types::*;
use tower_lsp_server::{LanguageServer, LspService};
use trainz_common::language_id::GAME_SCRIPT_LANGUAGE_ID;
use trainz_language_server::state::TrainzLanguageServer;

#[tokio::test]
async fn test_hover_method_call_signature() {
    let (service, _) = LspService::new(|client| {
        TrainzLanguageServer::new(client, None, vec![], "test-version", None, None, None)
    });

    let temp_dir = std::env::current_dir()
        .unwrap()
        .join("target")
        .join("test_hover_method_call_signature");
    if temp_dir.exists() {
        fs::remove_dir_all(&temp_dir).unwrap();
    }
    fs::create_dir_all(&temp_dir).unwrap();

    let file_path = temp_dir.join("Test.gs");
    let code = "class Test {\n  public void CountTags(int pid, string s) { } // counts tags in acs_text\n  void Run() {\n    CountTags(1, \"test\");\n  }\n};";
    fs::write(&file_path, code).unwrap();
    let uri = Uri::from_file_path(&file_path).unwrap();

    // Open file
    service
        .inner()
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: GAME_SCRIPT_LANGUAGE_ID.to_string(),
                version: 1,
                text: code.to_string(),
            },
        })
        .await;

    // Hover over 'CountTags' in 'Run' (line 3, character 4)
    let params = HoverParams {
        text_document_position_params: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            position: Position {
                line: 3,
                character: 4,
            },
        },
        work_done_progress_params: Default::default(),
    };

    let result = service.inner().hover(params).await.unwrap();
    assert!(result.is_some(), "Hover should return a result");

    if let Some(hover) = result {
        if let HoverContents::Markup(markup) = hover.contents {
            assert!(
                markup
                    .value
                    .contains("public void Test::CountTags(int pid, string s)"),
                "Hover should contain method signature. Got: {}",
                markup.value
            );
            assert!(
                markup.value.contains("counts tags in acs_text"),
                "Hover should contain method comment. Got: {}",
                markup.value
            );
        } else {
            panic!("Expected markup contents");
        }
    }

    let _ = fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_hover_cross_file_method_call() {
    let (service, _) = LspService::new(|client| {
        TrainzLanguageServer::new(client, None, vec![], "test-version", None, None, None)
    });

    let temp_dir = std::env::current_dir()
        .unwrap()
        .join("target")
        .join("test_hover_cross_file_method_call");
    if temp_dir.exists() {
        fs::remove_dir_all(&temp_dir).unwrap();
    }
    fs::create_dir_all(&temp_dir).unwrap();

    let main_path = temp_dir.join("Main.gs");
    let helper_path = temp_dir.join("Helper.gs");

    let main_code = "include \"Helper.gs\"\nclass Main {\n  void Run() {\n    Helper h = new Helper();\n    h.DoSomething();\n  }\n};";
    let helper_code =
        "class Helper {\n  // does something useful\n  public void DoSomething() {}\n};";

    fs::write(&main_path, main_code).unwrap();
    fs::write(&helper_path, helper_code).unwrap();

    let main_uri = Uri::from_file_path(&main_path).unwrap();
    let helper_uri = Uri::from_file_path(&helper_path).unwrap();

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

    // Open files
    service
        .inner()
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: helper_uri.clone(),
                language_id: GAME_SCRIPT_LANGUAGE_ID.to_string(),
                version: 1,
                text: helper_code.to_string(),
            },
        })
        .await;

    service
        .inner()
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: main_uri.clone(),
                language_id: GAME_SCRIPT_LANGUAGE_ID.to_string(),
                version: 1,
                text: main_code.to_string(),
            },
        })
        .await;

    // Hover over 'DoSomething' in 'Main.gs' (line 4, character 6)
    let params = HoverParams {
        text_document_position_params: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: main_uri.clone(),
            },
            position: Position {
                line: 4,
                character: 6,
            },
        },
        work_done_progress_params: Default::default(),
    };

    let result = service.inner().hover(params).await.unwrap();
    assert!(
        result.is_some(),
        "Hover should return a result for cross-file call"
    );

    if let Some(hover) = result {
        if let HoverContents::Markup(markup) = hover.contents {
            assert!(
                markup.value.contains("public void Helper::DoSomething()"),
                "Hover should contain cross-file method signature. Got: {}",
                markup.value
            );
            assert!(
                markup.value.contains("does something useful"),
                "Hover should contain cross-file method comment. Got: {}",
                markup.value
            );
        } else {
            panic!("Expected markup contents");
        }
    }

    let _ = fs::remove_dir_all(&temp_dir);
}
