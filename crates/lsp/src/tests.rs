use crate::state::GameScriptLanguageServer;
use tokio::time::{timeout, Duration};
use tower_lsp_server::ls_types::*;
use tower_lsp_server::{LanguageServer, LspService};

#[tokio::test]
async fn test_did_change_deadlock() {
    let (service, _) =
        LspService::new(|client| GameScriptLanguageServer::new(client, None, vec![], ""));

    let uri = Uri::from_file_path(
        std::env::current_dir()
            .unwrap()
            .join("test_programs/hello_world.gs"),
    )
    .unwrap();

    // Ensure the file exists on disk if the server checks for it
    let path = uri.to_file_path().unwrap();
    if !path.exists() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "").unwrap();
    }

    // 1. Open the file
    let content = "include \"Bar.gs\"\nclass Foo {};";
    let did_open_params = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "game-script".to_string(),
            version: 1,
            text: content.to_string(),
        },
    };
    service.inner().did_open(did_open_params).await;

    // 2. Change the file - this should NOT deadlock
    let did_change_params = DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
            uri: uri.clone(),
            version: 2,
        },
        content_changes: vec![TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: content.to_string() + "\n// some change",
        }],
    };

    // Use a timeout to detect deadlock
    let result = timeout(
        Duration::from_secs(5),
        service.inner().did_change(did_change_params),
    )
    .await;

    assert!(result.is_ok(), "did_change timed out - likely a deadlock!");
}

#[tokio::test]
async fn test_semantic_tokens_initial() {
    let (service, _) =
        LspService::new(|client| GameScriptLanguageServer::new(client, None, vec![], ""));
    let uri = Uri::from_file_path(
        std::env::current_dir()
            .unwrap()
            .join("test_programs/hello_world_initial.gs"),
    )
    .unwrap();

    // Ensure the file exists on disk if the server checks for it
    let path = uri.to_file_path().unwrap();
    if !path.exists() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "").unwrap();
    }

    let content = "include \"Bar.gs\"\nclass Foo {};";
    service
        .inner()
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "game-script".to_string(),
                version: 1,
                text: content.to_string(),
            },
        })
        .await;

    let params = SemanticTokensParams {
        text_document: TextDocumentIdentifier { uri: uri.clone() },
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
    };

    let result = service.inner().semantic_tokens_full(params).await.unwrap();

    if let Some(SemanticTokensResult::Tokens(tokens)) = result {
        // "include" keyword (type 0), "\"Bar.gs\"" string (type 2)
        // "class" keyword (rule: keyword_class, token_type: 0)
        // "Foo" class name (rule: class_name, token_type: 1)

        // Find "include"
        let include_token = tokens
            .data
            .iter()
            .find(|t| t.length == 7 && t.token_type == 0);
        assert!(
            include_token.is_some(),
            "Should find 'include' keyword. Tokens: {:?}",
            tokens.data
        );

        // Find "\"Bar.gs\""
        let path_token = tokens
            .data
            .iter()
            .find(|t| t.length == 8 && t.token_type == 2);
        assert!(path_token.is_some(), "Should find '\"Bar.gs\"' path");

        // Find "class"
        let class_token = tokens
            .data
            .iter()
            .find(|t| t.length == 5 && t.token_type == 0);
        assert!(class_token.is_some(), "Should find 'class' keyword");

        // Find "Foo"
        let foo_token = tokens
            .data
            .iter()
            .find(|t| t.length == 3 && t.token_type == 1);
        assert!(foo_token.is_some(), "Should find 'Foo' class name");
    } else {
        panic!("Expected semantic tokens result");
    }
}

#[tokio::test]
async fn test_semantic_tokens_statements_literals() {
    let (service, _) =
        LspService::new(|client| GameScriptLanguageServer::new(client, None, vec![], ""));
    let uri = Uri::from_file_path(
        std::env::current_dir()
            .unwrap()
            .join("test_programs/stmts.gs"),
    )
    .unwrap();

    let path = uri.to_file_path().unwrap();
    if !path.exists() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "").unwrap();
    }

    let content = "class Test { void Main() { int i = 123; if (i == 123) { return; } } };";
    service
        .inner()
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "game-script".to_string(),
                version: 1,
                text: content.to_string(),
            },
        })
        .await;

    let params = SemanticTokensParams {
        text_document: TextDocumentIdentifier { uri: uri.clone() },
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
    };

    let result = service.inner().semantic_tokens_full(params).await.unwrap();

    if let Some(SemanticTokensResult::Tokens(tokens)) = result {
        // int (type 3), i (type 10), 123 (type 6)
        // if (type 0), return (type 0)
        // VARIABLE is type 10 in legend

        let int_token = tokens
            .data
            .iter()
            .find(|t| t.length == 3 && t.token_type == 3);
        assert!(
            int_token.is_some(),
            "Should find 'int' type. Tokens: {:?}",
            tokens.data
        );

        let i_token = tokens
            .data
            .iter()
            .find(|t| t.length == 1 && t.token_type == 10);
        assert!(
            i_token.is_some(),
            "Should find 'i' variable. Tokens: {:?}",
            tokens.data
        );

        let num_token = tokens
            .data
            .iter()
            .find(|t| t.length == 3 && t.token_type == 6);
        assert!(num_token.is_some(), "Should find '123' number");

        let if_token = tokens
            .data
            .iter()
            .find(|t| t.length == 2 && t.token_type == 0);
        assert!(if_token.is_some(), "Should find 'if' keyword");

        let return_token = tokens
            .data
            .iter()
            .find(|t| t.length == 6 && t.token_type == 0);
        assert!(return_token.is_some(), "Should find 'return' keyword");

        let block_tokens: Vec<_> = tokens.data.iter().filter(|t| t.token_type == 13).collect();
        assert!(
            block_tokens.is_empty(),
            "Should NOT find macro block tokens"
        );
    } else {
        panic!("Expected semantic tokens result");
    }
}

#[tokio::test]
async fn test_semantic_tokens_update() {
    let (service, _) = LspService::new(|client| {
        GameScriptLanguageServer::new(client, None, vec![], "test-version")
    });
    let uri = Uri::from_file_path(
        std::env::current_dir()
            .unwrap()
            .join("test_programs/hello_world_update.gs"),
    )
    .unwrap();

    // Ensure the file exists on disk if the server checks for it
    let path = uri.to_file_path().unwrap();
    if !path.exists() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "").unwrap();
    }

    // 1. Initial content
    let content1 = "class Foo {};";
    service
        .inner()
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "game-script".to_string(),
                version: 1,
                text: content1.to_string(),
            },
        })
        .await;

    // 2. Change content to include more tokens
    let content2 = "class Foo {\n  public void Bar() {}\n};";
    service
        .inner()
        .did_change(DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier {
                uri: uri.clone(),
                version: 2,
            },
            content_changes: vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: content2.to_string(),
            }],
        })
        .await;

    let params = SemanticTokensParams {
        text_document: TextDocumentIdentifier { uri: uri.clone() },
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
    };

    let result = service.inner().semantic_tokens_full(params).await.unwrap();

    if let Some(SemanticTokensResult::Tokens(tokens)) = result {
        // Tokens should include: class, Foo, public, void, Bar
        // "class" at (0,0) - index 0
        // "Foo" at (0,6) - index 1
        // "public" at (1, 2) - index 9 (MODIFIER)
        // "void" at (1, 9) - index 3 (TYPE)
        // "Bar" at (1, 14) - index 4 (METHOD)

        let bar_token = tokens.data.iter().find(|t| t.token_type == 4); // METHOD is 4
        assert!(
            bar_token.is_some(),
            "Should find 'Bar' method token. Tokens: {:?}",
            tokens.data
        );

        let public_token = tokens
            .data
            .iter()
            .find(|t| t.length == 6 && t.token_type == 9); // MODIFIER is 9
        assert!(
            public_token.is_some(),
            "Should find 'public' modifier token"
        );
    } else {
        panic!("Expected semantic tokens result after update");
    }
}

#[tokio::test]
async fn test_semantic_tokens_soup() {
    let (service, _) =
        LspService::new(|client| GameScriptLanguageServer::new(client, None, vec![], ""));
    let uri = Uri::from_file_path(
        std::env::current_dir()
            .unwrap()
            .join("test_programs/config.txt"),
    )
    .unwrap();

    // Ensure the file exists on disk
    let path = uri.to_file_path().unwrap();
    if !path.exists() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "").unwrap();
    }

    let content = "kuid <kuid:123:456>\nusername \"test\"";
    service
        .inner()
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "soup".to_string(),
                version: 1,
                text: content.to_string(),
            },
        })
        .await;

    let params = SemanticTokensParams {
        text_document: TextDocumentIdentifier { uri: uri.clone() },
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
    };

    let result = service.inner().semantic_tokens_full(params).await.unwrap();

    if let Some(SemanticTokensResult::Tokens(tokens)) = result {
        // "kuid" key (rule: key, token_type: 0)
        // "<kuid:123:456>" value (rule: kuid_value, token_type: 7)
        // "username" key (rule: key, token_type: 0)
        // "\"test\"" string (rule: string, token_type: 2)

        let kuid_key = tokens
            .data
            .iter()
            .find(|t| t.token_type == 0 && t.length == 4);
        assert!(kuid_key.is_some(), "Should find 'kuid' key token");

        let kuid_value = tokens.data.iter().find(|t| t.token_type == 7);
        assert!(kuid_value.is_some(), "Should find kuid value token");

        let username_key = tokens
            .data
            .iter()
            .find(|t| t.token_type == 0 && t.length == 8);
        assert!(username_key.is_some(), "Should find 'username' key token");
    } else {
        panic!("Expected semantic tokens result for soup file");
    }
}

#[tokio::test]
async fn test_semantic_tokens_isclass() {
    let (service, _) =
        LspService::new(|client| GameScriptLanguageServer::new(client, None, vec![], ""));
    let uri = Uri::from_file_path(
        std::env::current_dir()
            .unwrap()
            .join("test_programs/hello_world_isclass.gs"),
    )
    .unwrap();

    let content = "class Foo isclass Bar { void Bar() { bool b; b = me.isclass(Foo); } };";
    let path = uri.to_file_path().unwrap();
    if !path.exists() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, content).unwrap();
    }
    service
        .inner()
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "game-script".to_string(),
                version: 1,
                text: content.to_string(),
            },
        })
        .await;

    let params = SemanticTokensParams {
        text_document: TextDocumentIdentifier { uri: uri.clone() },
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
    };

    let result = service.inner().semantic_tokens_full(params).await.unwrap();

    if let Some(SemanticTokensResult::Tokens(tokens)) = result {
        // "class" (type 0), "Foo" (type 1), "isclass" (type 10), "Bar" (type 1)

        let class_token = tokens
            .data
            .iter()
            .find(|t| t.length == 5 && t.token_type == 0);
        assert!(class_token.is_some(), "Should find 'class' keyword token");

        let isclass_inheritance_token = tokens
            .data
            .iter()
            .find(|t| t.length == 7 && t.token_type == 0);
        assert!(
            isclass_inheritance_token.is_some(),
            "Should find 'isclass' keyword token in inheritance. Tokens: {:?}",
            tokens.data
        );

        let isclass_check_token = tokens
            .data
            .iter()
            .find(|t| t.length == 7 && t.token_type == 0 && t.delta_start > 0);
        assert!(
            isclass_check_token.is_some(),
            "Should find 'isclass' keyword token in check. Tokens: {:?}",
            tokens.data
        );
    } else {
        panic!("Expected semantic tokens result");
    }
}

#[tokio::test]
async fn test_semantic_tokens_include() {
    let (service, _) =
        LspService::new(|client| GameScriptLanguageServer::new(client, None, vec![], ""));
    let uri = Uri::from_file_path(
        std::env::current_dir()
            .unwrap()
            .join("test_programs/include_test.gs"),
    )
    .unwrap();

    let content = "include \"Subdir/Helper.gs\"\ninclude \"Other.gs\"\nclass Foo {};";
    let path = uri.to_file_path().unwrap();
    if !path.exists() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, content).unwrap();
    }
    service
        .inner()
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "game-script".to_string(),
                version: 1,
                text: content.to_string(),
            },
        })
        .await;

    let params = SemanticTokensParams {
        text_document: TextDocumentIdentifier { uri: uri.clone() },
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
    };

    let result = service.inner().semantic_tokens_full(params).await.unwrap();

    if let Some(SemanticTokensResult::Tokens(tokens)) = result {
        // Find both "include" tokens
        let include_tokens: Vec<_> = tokens
            .data
            .iter()
            .filter(|t| t.length == 7 && t.token_type == 0)
            .collect();
        assert_eq!(
            include_tokens.len(),
            2,
            "Should find 2 'include' keyword tokens. Tokens: {:?}",
            tokens.data
        );

        // Find "Subdir/Helper.gs"
        // Length of "\"Subdir/Helper.gs\"" is 18
        let helper_path = tokens
            .data
            .iter()
            .find(|t| t.length == 18 && t.token_type == 2);
        assert!(
            helper_path.is_some(),
            "Should find '\"Subdir/Helper.gs\"' path"
        );

        // Find "Other.gs"
        // Length of "\"Other.gs\"" is 10
        let other_path = tokens
            .data
            .iter()
            .find(|t| t.length == 10 && t.token_type == 2);
        assert!(other_path.is_some(), "Should find '\"Other.gs\"' path");
    } else {
        panic!("Expected semantic tokens result");
    }
}

#[tokio::test]
async fn test_soup_diagnostics() {
    let temp_dir = std::env::current_dir()
        .unwrap()
        .join("temp_validation_test");
    std::fs::create_dir_all(&temp_dir).unwrap();

    // Test Case 1: category-era.txt with semicolon separated values
    let era_path = temp_dir.join("category-era.txt");
    std::fs::write(
        &era_path,
        "era1 \"Era 1 Description\"\nera2 \"Era 2 Description\"",
    )
    .unwrap();

    // Test Case 2: category-region.txt with special key '00'
    let region_path = temp_dir.join("category-region.txt");
    std::fs::write(&region_path, "00 \"No Region\"\nAU \"Australia\"").unwrap();

    // Add a container validator for testing category-era
    let container_path = temp_dir.join("my_container.txt");
    let container_rules = r#"
my_container {
    key {
        type "string"
        validation "IsValidCategoryEra"
    }
    region {
        type "string"
        validation "IsValidCategoryRegion"
    }
}
"#;
    std::fs::write(&container_path, container_rules).unwrap();

    let (service, _) = LspService::new(|client| {
        GameScriptLanguageServer::new(client, Some(temp_dir.clone()), vec![], "")
    });

    service.inner().initialized(InitializedParams {}).await;

    // content with a key matching the validator filename and a value
    let content = "my_container\n{\n  key \"era1;invalid_era\"\n  region \"00\"\n}\n";
    let uri = Uri::from_file_path(temp_dir.join("test_file.soup")).unwrap();
    let path = uri.to_file_path().unwrap();
    std::fs::write(&path, content).unwrap();

    service
        .inner()
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "soup".to_string(),
                version: 1,
                text: content.to_string(),
            },
        })
        .await;

    // Trigger diagnostic calculation
    let params = DocumentDiagnosticParams {
        text_document: TextDocumentIdentifier { uri: uri.clone() },
        identifier: None,
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
        previous_result_id: None,
    };

    let result = service.inner().diagnostic(params).await.unwrap();

    if let DocumentDiagnosticReportResult::Report(report) = result {
        match report {
            DocumentDiagnosticReport::Full(full) => {
                let items = &full.full_document_diagnostic_report.items;

                let has_era_error = items.iter().any(|diag| {
                    diag.message
                        .contains("Value 'invalid_era' for key 'key' is not a valid category era.")
                        && diag.severity == Some(DiagnosticSeverity::ERROR)
                });
                assert!(
                    has_era_error,
                    "Should find diagnostic for invalid 'invalid_era'. Diagnostics: {:?}",
                    items
                );

                let has_region_error = items.iter().any(|diag| diag.message.contains("region"));
                assert!(
                    !has_region_error,
                    "Should NOT find diagnostic for valid '00' in category-region. Diagnostics: {:?}",
                    items
                );
            }
            DocumentDiagnosticReport::Unchanged(_) => {
                panic!("Expected full diagnostic report, got unchanged");
            }
        }
    } else {
        panic!("Expected report result, got {:?}", result);
    }

    // Test Completions (era)
    let completion_params_era = CompletionParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            position: Position {
                line: 2,
                character: 10,
            }, // middle of "era1;invalid_era"
        },
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
        context: None,
    };

    let completions_era = service
        .inner()
        .completion(completion_params_era)
        .await
        .unwrap()
        .unwrap();
    let items_era = match completions_era {
        CompletionResponse::Array(items) => items,
        CompletionResponse::List(list) => list.items,
    };

    let has_era1 = items_era
        .iter()
        .any(|item| item.label == "era1" && item.detail == Some("Era 1 Description".to_string()));
    let has_era2 = items_era
        .iter()
        .any(|item| item.label == "era2" && item.detail == Some("Era 2 Description".to_string()));
    assert!(
        has_era1,
        "Should find 'era1' in completions. Completions: {:?}",
        items_era
    );
    assert!(
        has_era2,
        "Should find 'era2' in completions. Completions: {:?}",
        items_era
    );

    // Test Region Completions
    let region_completion_params = CompletionParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            position: Position {
                line: 3,
                character: 10,
            }, // middle of "00"
        },
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
        context: None,
    };

    let region_completions = service
        .inner()
        .completion(region_completion_params)
        .await
        .unwrap()
        .unwrap();
    let region_items = match region_completions {
        CompletionResponse::Array(items) => items,
        CompletionResponse::List(list) => list.items,
    };

    let has_00 = region_items
        .iter()
        .any(|item| item.label == "00" && item.detail == Some("No Region".to_string()));
    assert!(
        has_00,
        "Should find '00' in region completions. Completions: {:?}",
        region_items
    );

    std::fs::remove_dir_all(temp_dir).unwrap();
}

#[tokio::test]
async fn test_thumbnails_validation_extended() {
    let temp_dir = std::env::current_dir()
        .unwrap()
        .join("temp_thumbnails_extended_test");
    if temp_dir.exists() {
        std::fs::remove_dir_all(&temp_dir).unwrap();
    }
    std::fs::create_dir_all(&temp_dir).unwrap();

    // Create container.txt with thumbnails and thumbnails-element
    let container_content = r#"
thumbnails
{
  icon thumbnails
  menu-token "$ccp_thumbnails_menu-name"
  description "Thumbnails Container" 
  unique 1
  default-amount 1
  kind "structure"
  array-element
  {
    container-type0 "thumbnails-element"
  }
  validation
  {
    UniqueNames
  } 
}

thumbnails-element
{
  image
  {
    type "string"
    description "Thumbnail Image Path"
  }
  width
  {
    type "numeric"
    description "Thumbnail Width"
  }
  height
  {
    type "numeric"
    description "Thumbnail Height"
  }
}
"#;
    std::fs::write(temp_dir.join("container.txt"), container_content).unwrap();

    let (service, _) = LspService::new(|client| {
        GameScriptLanguageServer::new(client, Some(temp_dir.clone()), vec![], "")
    });

    service.inner().initialized(InitializedParams {}).await;

    let uri = Uri::from_file_path(
        std::env::current_dir()
            .unwrap()
            .join("test_programs/test_thumbnails_extended.txt"),
    )
    .unwrap();

    let content = r#"thumbnails {
  preview {
    image "icon/icon.jpg"
    width "invalid_width"
    height 180
  }
  main {
    image "main.jpg"
    width 800
  }
}
"#;
    let path = uri.to_file_path().unwrap();
    if !path.exists() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    }
    std::fs::write(&path, content).unwrap();

    service
        .inner()
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "soup".to_string(),
                version: 1,
                text: content.to_string(),
            },
        })
        .await;

    // 1. Check Diagnostics for type mismatch in nested element
    let params = DocumentDiagnosticParams {
        text_document: TextDocumentIdentifier { uri: uri.clone() },
        identifier: None,
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
        previous_result_id: None,
    };

    let result = service.inner().diagnostic(params).await.unwrap();

    if let DocumentDiagnosticReportResult::Report(report) = result {
        match report {
            DocumentDiagnosticReport::Full(full) => {
                let items = &full.full_document_diagnostic_report.items;

                // Should find type error for 'width' (expected numeric, found string)
                let has_type_error = items.iter().any(|diag| {
                    diag.message
                        .contains("Invalid type for key 'thumbnails-element' in container 'width'. Expected 'numeric', found 'string', kind 'None'")
                        && diag.severity == Some(DiagnosticSeverity::ERROR)
                });
                assert!(
                    has_type_error,
                    "Should find type error for 'width' in 'thumbnails-element'. Diagnostics: {:?}",
                    items
                );

                // Should NOT find duplicate key error (since 'preview' and 'main' are unique)
                let has_duplicate_error = items
                    .iter()
                    .any(|diag| diag.message.contains("Duplicate key"));
                assert!(
                    !has_duplicate_error,
                    "Should NOT find duplicate key errors. Diagnostics: {:?}",
                    items
                );
            }
            _ => panic!("Expected full report"),
        }
    }

    // 2. Check Completions inside a nested element
    let completion_params = CompletionParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            position: Position {
                line: 8,
                character: 4,
            }, // inside 'main' block
        },
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
        context: None,
    };

    let completions = service
        .inner()
        .completion(completion_params)
        .await
        .unwrap()
        .unwrap();
    let items = match completions {
        CompletionResponse::Array(items) => items,
        CompletionResponse::List(list) => list.items,
    };

    let has_image = items.iter().any(|item| item.label == "image");
    let has_height = items.iter().any(|item| item.label == "height");
    assert!(has_image, "Should suggest 'image' inside 'main'");
    assert!(has_height, "Should suggest 'height' inside 'main'");

    // 3. Check Hover on a nested field
    let hover_params = HoverParams {
        text_document_position_params: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            position: Position {
                line: 2,
                character: 5,
            }, // over "image" in 'preview'
        },
        work_done_progress_params: Default::default(),
    };

    let hover = service.inner().hover(hover_params).await.unwrap().unwrap();
    if let HoverContents::Markup(markup) = hover.contents {
        assert!(
            markup.value.contains("Key: `image`"),
            "Hover text missing key name: {}",
            markup.value
        );
        assert!(
            markup.value.contains("Thumbnail Image Path"),
            "Hover text missing description: {}",
            markup.value
        );
    } else {
        panic!("Expected markup hover contents");
    }

    std::fs::remove_dir_all(temp_dir).unwrap();
    std::fs::remove_file(path).unwrap();
}

#[tokio::test]
async fn test_thumbnails_container_validation() {
    let temp_dir = std::env::current_dir()
        .unwrap()
        .join("temp_thumbnails_test");
    if temp_dir.exists() {
        std::fs::remove_dir_all(&temp_dir).unwrap();
    }
    std::fs::create_dir_all(&temp_dir).unwrap();

    // Create container.txt with thumbnails and thumbnails-element
    let container_content = r#"
thumbnails
{
  icon thumbnails
  menu-token "$ccp_thumbnails_menu-name"
  description "" 
  unique 1
  default-amount 1
  kind "structure"
  array-element
  {
    container-type0 "thumbnails-element"
  }
  validation
  {
    UniqueNames
  } 
}

thumbnails-element
{
  image
  {
    type "string"
  }
  width
  {
    type "numeric"
  }
  height
  {
    type "numeric"
  }
}
"#;
    std::fs::write(temp_dir.join("container.txt"), container_content).unwrap();

    let (service, _) = LspService::new(|client| {
        GameScriptLanguageServer::new(client, Some(temp_dir.clone()), vec![], "")
    });

    service.inner().initialized(InitializedParams {}).await;

    let uri = Uri::from_file_path(
        std::env::current_dir()
            .unwrap()
            .join("test_programs/test_thumbnails.txt"),
    )
    .unwrap();

    let content = r#"thumbnails {
  preview {
    image "icon/icon.jpg"
    width 240
    height 180
  }
  duplicate {
    image "dup.jpg"
  }
  preview {
    image "error"
  }
}
"#;
    let path = uri.to_file_path().unwrap();
    if !path.exists() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    }
    std::fs::write(&path, content).unwrap();

    service
        .inner()
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "soup".to_string(),
                version: 1,
                text: content.to_string(),
            },
        })
        .await;

    // Check Diagnostics
    let params = DocumentDiagnosticParams {
        text_document: TextDocumentIdentifier { uri: uri.clone() },
        identifier: None,
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
        previous_result_id: None,
    };

    let result = service.inner().diagnostic(params).await.unwrap();

    if let DocumentDiagnosticReportResult::Report(report) = result {
        match report {
            DocumentDiagnosticReport::Full(full) => {
                let items = &full.full_document_diagnostic_report.items;

                // Should find duplicate key 'preview'
                let has_duplicate_error = items.iter().any(|diag| {
                    diag.message
                        .contains("Duplicate key 'preview' in container 'thumbnails'")
                        && diag.severity == Some(DiagnosticSeverity::ERROR)
                });
                assert!(
                    has_duplicate_error,
                    "Should find duplicate key error for 'preview'. Diagnostics: {:?}",
                    items
                );

                // Check that 'thumbnails-element' validation works (e.g., inside 'duplicate')
                // No errors should be reported for 'duplicate' if it matches 'thumbnails-element'
                let has_duplicate_content_error = items.iter().any(|diag| {
                    diag.range.start.line == 7 // inside 'duplicate'
                });
                assert!(
                    !has_duplicate_content_error,
                    "Should NOT find error inside 'duplicate' container. Diagnostics: {:?}",
                    items
                );
            }
            _ => panic!("Expected full report"),
        }
    }

    std::fs::remove_dir_all(temp_dir).unwrap();
}

#[tokio::test]
async fn test_container_validation() {
    let temp_dir = std::env::current_dir().unwrap().join("temp_container_test");
    std::fs::create_dir_all(&temp_dir).unwrap();

    // Create container.txt
    let container_content = r#"
my-container
{
  key1
  {
    type "string"
    validation "v1,v2"
  }
  key2
  {
    type "numeric"
  }
}
"#;
    std::fs::write(temp_dir.join("container.txt"), container_content).unwrap();

    // Create effect-layer.txt
    let effect_content = r#"
layer1
{
  alpha
  {
    type "numeric"
    validation "0,1"
  }
}
"#;
    std::fs::write(temp_dir.join("effect-layer.txt"), effect_content).unwrap();

    // Create category-era.txt
    let era_content = r#"
era1 "Era 1 Description"
era2 "Era 2 Description"
"#;
    std::fs::write(temp_dir.join("category-era.txt"), era_content).unwrap();

    let (service, _) = LspService::new(|client| {
        GameScriptLanguageServer::new(client, Some(temp_dir.clone()), vec![], "")
    });

    service.inner().initialized(InitializedParams {}).await;

    let uri = Uri::from_file_path(
        std::env::current_dir()
            .unwrap()
            .join("test_programs/test_container.txt"),
    )
    .unwrap();

    let content = r#"my-container
{
  key1 "v1"
  key2 "invalid"
  unknown "key"
}
layer1
{
  alpha 0
}
category-era "era1;invalid"
"#;
    let path = uri.to_file_path().unwrap();
    if !path.exists() {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    }
    std::fs::write(&path, content).unwrap();

    service
        .inner()
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "soup".to_string(),
                version: 1,
                text: content.to_string(),
            },
        })
        .await;

    // 1. Check Diagnostics
    let params = DocumentDiagnosticParams {
        text_document: TextDocumentIdentifier { uri: uri.clone() },
        identifier: None,
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
        previous_result_id: None,
    };

    let result = service.inner().diagnostic(params).await.unwrap();

    if let DocumentDiagnosticReportResult::Report(report) = result {
        match report {
            DocumentDiagnosticReport::Full(full) => {
                let items = &full.full_document_diagnostic_report.items;

                // key2 should have a type error (expected numeric, found string)
                let has_type_error = items.iter().any(|diag| {
                    diag.message
                        .contains("Invalid type for key 'key2' in container 'my-container'")
                        && diag.severity == Some(DiagnosticSeverity::ERROR)
                });
                assert!(
                    has_type_error,
                    "Should find type error for 'key2'. Diagnostics: {:?}",
                    items
                );

                // unknown should have a warning
                let has_unknown_warning = items.iter().any(|diag| {
                    diag.message
                        .contains("Unknown key 'unknown' in container 'my-container'")
                        && diag.severity == Some(DiagnosticSeverity::WARNING)
                });
                assert!(
                    has_unknown_warning,
                    "Should find unknown key warning. Diagnostics: {:?}",
                    items
                );

                // alpha should be valid
                let has_alpha_error = items.iter().any(|diag| diag.message.contains("alpha"));
                assert!(
                    !has_alpha_error,
                    "Should NOT find error for 'alpha'. Diagnostics: {:?}",
                    items
                );
            }
            _ => panic!("Expected full report"),
        }
    }

    // 2. Check Completions inside my-container (keys)
    let completion_params = CompletionParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            position: Position {
                line: 3,
                character: 2,
            }, // inside my-container
        },
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
        context: None,
    };

    let completions = service
        .inner()
        .completion(completion_params)
        .await
        .unwrap()
        .unwrap();
    let items = match completions {
        CompletionResponse::Array(items) => items,
        CompletionResponse::List(list) => list.items,
    };

    let has_key1 = items.iter().any(|item| item.label == "key1");
    let has_key2 = items.iter().any(|item| item.label == "key2");
    assert!(has_key1, "Should suggest 'key1' in my-container");
    assert!(has_key2, "Should suggest 'key2' in my-container");

    // 3. Check Completions for key1 value
    let val_completion_params = CompletionParams {
        text_document_position: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            position: Position {
                line: 2,
                character: 8,
            }, // inside "v1"
        },
        work_done_progress_params: Default::default(),
        partial_result_params: Default::default(),
        context: None,
    };

    let val_completions = service
        .inner()
        .completion(val_completion_params)
        .await
        .unwrap()
        .unwrap();
    let val_items = match val_completions {
        CompletionResponse::Array(items) => items,
        CompletionResponse::List(list) => list.items,
    };

    let has_v1 = val_items.iter().any(|item| item.label == "v1");
    let has_v2 = val_items.iter().any(|item| item.label == "v2");
    assert!(has_v1, "Should suggest 'v1' for key1");
    assert!(has_v2, "Should suggest 'v2' for key1");

    // 4. Check Hover for key1
    let hover_params = HoverParams {
        text_document_position_params: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            position: Position {
                line: 2,
                character: 3,
            }, // over "key1"
        },
        work_done_progress_params: Default::default(),
    };

    let hover = service.inner().hover(hover_params).await.unwrap().unwrap();
    if let HoverContents::Markup(markup) = hover.contents {
        assert!(markup.value.contains("Key: `key1`"));
        assert!(markup.value.contains("**Validation**: `v1,v2`"));
    } else {
        panic!("Expected markup hover contents");
    }

    // 5. Check Hover for category-era value
    let era_hover_params = HoverParams {
        text_document_position_params: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            position: Position {
                line: 10,
                character: 14,
            }, // over "era1" in category-era
        },
        work_done_progress_params: Default::default(),
    };

    let era_hover = service.inner().hover(era_hover_params).await.unwrap();
    assert!(era_hover.is_some(), "Expected hover for era1");
    if let HoverContents::Markup(markup) = era_hover.unwrap().contents {
        assert!(
            markup.value.contains("**Value**: `era1`"),
            "Value not found in hover markup: {}",
            markup.value
        );
        assert!(
            markup.value.contains("**Description**: Era 1 Description"),
            "Description not found in hover markup: {}",
            markup.value
        );
    } else {
        panic!("Expected markup hover contents");
    }

    std::fs::remove_dir_all(temp_dir).unwrap();
    std::fs::remove_file(path).unwrap();
}
