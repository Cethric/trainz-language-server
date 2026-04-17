use rayon::prelude::*;
use std::fs;
use std::path::PathBuf;
use tower_lsp_server::ls_types::*;
use tower_lsp_server::{LanguageServer, LspService};
use trainz_common::language_id::GAME_SCRIPT_LANGUAGE_ID;
use trainz_language_server::state::GameScriptLanguageServer;

fn setup_temp_workspace(test_name: &str) -> (PathBuf, Uri) {
    let temp_dir = std::env::current_dir()
        .unwrap()
        .join("target")
        .join(test_name);
    if temp_dir.exists() {
        fs::remove_dir_all(&temp_dir).unwrap();
    }
    fs::create_dir_all(&temp_dir).unwrap();

    let main_path = temp_dir.join("Main.gs");
    let helper_path = temp_dir.join("Helper.gs");

    fs::write(&main_path, "include \"Helper.gs\"\nclass Main {\n  public void Run() {\n    Helper h = new Helper();\n    h.DoSomething();\n  }\n};").unwrap();
    fs::write(
        &helper_path,
        "class Helper {\n  public void DoSomething() {}\n};",
    )
    .unwrap();

    let main_uri = Uri::from_file_path(&main_path).unwrap();
    (temp_dir, main_uri)
}

#[tokio::test]
async fn test_include_document_symbols() {
    let (service, _) = LspService::new(|client| {
        GameScriptLanguageServer::new(client, None, vec![], "test-version", None, None)
    });
    let (temp_dir, main_uri) = setup_temp_workspace("test_include_document_symbols");

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

    // Open Main.gs
    service
        .inner()
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: main_uri.clone(),
                language_id: GAME_SCRIPT_LANGUAGE_ID.to_string(),
                version: 1,
                text: fs::read_to_string(main_uri.to_file_path().unwrap()).unwrap(),
            },
        })
        .await;

    let params = DocumentSymbolParams {
        text_document: TextDocumentIdentifier {
            uri: main_uri.clone(),
        },
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
    };

    let result = service.inner().document_symbol(params).await.unwrap();

    if let Some(DocumentSymbolResponse::Nested(mut symbols)) = result {
        if symbols.len() == 1 && symbols[0].name == "file" {
            symbols = symbols.remove(0).children.unwrap_or_default();
        }

        // Find include symbol
        let include_symbol = symbols
            .par_iter()
            .find_first(|s| s.name == "Helper.gs" && s.kind == SymbolKind::MODULE);
        assert!(
            include_symbol.is_some(),
            "Should find 'Helper.gs' include symbol. Symbols: {:?}",
            symbols
        );

        // Find class symbol
        let class_symbol = symbols
            .par_iter()
            .find_first(|s| s.name == "Main" && s.kind == SymbolKind::CLASS);
        assert!(class_symbol.is_some(), "Should find 'Main' class symbol");
    } else {
        panic!("Expected nested document symbols");
    }

    let _ = fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_include_goto_definition() {
    let (service, _) = LspService::new(|client| {
        GameScriptLanguageServer::new(client, None, vec![], "test-version", None, None)
    });
    let (temp_dir, main_uri) = setup_temp_workspace("test_include_goto_definition");
    let helper_uri = Uri::from_file_path(temp_dir.join("Helper.gs")).unwrap();

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

    // Open Helper.gs first so it's parsed
    service
        .inner()
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: helper_uri.clone(),
                language_id: GAME_SCRIPT_LANGUAGE_ID.to_string(),
                version: 1,
                text: fs::read_to_string(helper_uri.to_file_path().unwrap()).unwrap(),
            },
        })
        .await;

    // Open Main.gs
    service
        .inner()
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: main_uri.clone(),
                language_id: GAME_SCRIPT_LANGUAGE_ID.to_string(),
                version: 1,
                text: fs::read_to_string(main_uri.to_file_path().unwrap()).unwrap(),
            },
        })
        .await;

    // 1. Goto definition on the include path "Helper.gs" (line 0, col 8-19)
    // include "Helper.gs"
    // 012345678
    let params = GotoDefinitionParams {
        text_document_position_params: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: main_uri.clone(),
            },
            position: Position {
                line: 0,
                character: 10,
            },
        },
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
    };

    let result = service.inner().goto_definition(params).await.unwrap();
    if let Some(GotoDefinitionResponse::Link(links)) = result {
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target_uri, helper_uri);
    } else {
        panic!(
            "Expected GotoDefinitionResponse::Link for include path, got {:?}",
            result
        );
    }

    // 2. Goto definition on the class "Helper" inside Main.gs (line 3, col 7)
    // include "Helper.gs"
    // class Main {
    //   public void Run() {
    //     Helper h = new Helper();
    // 01234567
    // Position line: 3, character: 7 is in 'Helper' in 'Helper h'
    let params = GotoDefinitionParams {
        text_document_position_params: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier {
                uri: main_uri.clone(),
            },
            position: Position {
                line: 3,
                character: 7,
            },
        },
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
    };

    let result = service.inner().goto_definition(params).await.unwrap();
    if let Some(GotoDefinitionResponse::Link(links)) = result {
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target_uri, helper_uri);
    } else {
        // Fallback: search symbols for "Helper" and use that position
        let sym_params = DocumentSymbolParams {
            text_document: TextDocumentIdentifier {
                uri: main_uri.clone(),
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };
        let sym_result = service.inner().document_symbol(sym_params).await.unwrap();
        if let Some(DocumentSymbolResponse::Nested(symbols)) = sym_result {
            // Find class Main, then method Run, then variable h
            let main = symbols.par_iter().find_first(|s| s.name == "Main").unwrap();
            let run = main
                .children
                .as_ref()
                .unwrap()
                .par_iter()
                .find_first(|s| s.name == "Run")
                .unwrap();
            let h = run
                .children
                .as_ref()
                .unwrap()
                .par_iter()
                .find_first(|s| s.name == "h")
                .unwrap();

            // The detail for 'h' contains the Helper type with its range.
            // detail: Some("Named(Identifier { name: \"Helper\", range: Range { start: Position { line: 3, character: 4 }, end: Position { line: 3, character: 10 } } })")
            // Let's use character 7 (middle of 4-10)
            let pos = Position {
                line: 3,
                character: 7,
            };
            let params = GotoDefinitionParams {
                text_document_position_params: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier {
                        uri: main_uri.clone(),
                    },
                    position: pos,
                },
                work_done_progress_params: Default::default(),
                partial_result_params: Default::default(),
            };
            let final_result = service.inner().goto_definition(params).await.unwrap();
            if let Some(GotoDefinitionResponse::Link(links)) = final_result {
                assert_eq!(links.len(), 1);
                assert_eq!(links[0].target_uri, helper_uri);
            } else {
                panic!(
                    "Final attempt failed. Position used: {:?}. Result: {:?}. Detail of h: {:?}. Helper URI: {:?}",
                    pos, final_result, h.detail, helper_uri
                );
            }
        } else {
            panic!("Symbols not found as nested");
        }
    }

    let _ = fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_include_document_links() {
    let (service, _) = LspService::new(|client| {
        GameScriptLanguageServer::new(client, None, vec![], "test-version", None, None)
    });
    let (temp_dir, main_uri) = setup_temp_workspace("test_include_document_links");
    let helper_uri = Uri::from_file_path(temp_dir.join("Helper.gs")).unwrap();

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

    // Open Main.gs
    service
        .inner()
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: main_uri.clone(),
                language_id: GAME_SCRIPT_LANGUAGE_ID.to_string(),
                version: 1,
                text: fs::read_to_string(main_uri.to_file_path().unwrap()).unwrap(),
            },
        })
        .await;

    let params = DocumentLinkParams {
        text_document: TextDocumentIdentifier {
            uri: main_uri.clone(),
        },
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
    };

    let result = service.inner().document_link(params).await.unwrap();

    if let Some(links) = result {
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target, Some(helper_uri));
        // Range for include "Helper.gs"
        assert_eq!(links[0].range.start.line, 0);
    } else {
        panic!("Expected document links");
    }

    let _ = fs::remove_dir_all(&temp_dir);
}
