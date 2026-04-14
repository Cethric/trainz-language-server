use std::fs;
use tower_lsp_server::ls_types::*;
use tower_lsp_server::{LanguageServer, LspService};
use trainz_language_server::state::GameScriptLanguageServer;

#[tokio::test]
async fn test_hover_same_line_comment() {
    let (service, _) = LspService::new(|client| {
        GameScriptLanguageServer::new(client, None, vec![], "test-version")
    });

    let temp_dir = std::env::current_dir()
        .unwrap()
        .join("target")
        .join("test_hover_same_line_comment");
    if temp_dir.exists() {
        fs::remove_dir_all(&temp_dir).unwrap();
    }
    fs::create_dir_all(&temp_dir).unwrap();

    let file_path = temp_dir.join("Test.gs");
    let code = "class Test {\n  public define int ERROR_INVALID_STATE = 2;    // Invalid state for query\n\n  public int GetError() { return ERROR_INVALID_STATE; }\n};";
    fs::write(&file_path, code).unwrap();
    let uri = Uri::from_file_path(&file_path).unwrap();

    // Open file
    service
        .inner()
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "game-script".to_string(),
                version: 1,
                text: code.to_string(),
            },
        })
        .await;

    // Hover over 'ERROR_INVALID_STATE' in 'GetError'
    let params = HoverParams {
        text_document_position_params: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            position: Position {
                line: 3,
                character: 35,
            },
        },
        work_done_progress_params: Default::default(),
    };

    let result = service.inner().hover(params).await.unwrap();
    assert!(result.is_some(), "Hover should return a result");

    if let Some(hover) = result {
        if let HoverContents::Markup(markup) = hover.contents {
            assert!(
                markup.value.contains("int Test::ERROR_INVALID_STATE = 2"),
                "Hover should contain declaration. Got: {}",
                markup.value
            );
            assert!(
                markup.value.contains("Invalid state for query"),
                "Hover should contain same-line comment. Got: {}",
                markup.value
            );
        } else {
            panic!("Expected markup contents");
        }
    }

    let _ = fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_hover_both_comments() {
    let (service, _) = LspService::new(|client| {
        GameScriptLanguageServer::new(client, None, vec![], "test-version")
    });

    let temp_dir = std::env::current_dir()
        .unwrap()
        .join("target")
        .join("test_hover_both_comments");
    if temp_dir.exists() {
        fs::remove_dir_all(&temp_dir).unwrap();
    }
    fs::create_dir_all(&temp_dir).unwrap();

    let file_path = temp_dir.join("Test.gs");
    let code = "class Test {\n  // Preceding comment\n  int m_field;    // Following comment\n\n  public int GetField() { return m_field; }\n};";
    fs::write(&file_path, code).unwrap();
    let uri = Uri::from_file_path(&file_path).unwrap();

    service
        .inner()
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "game-script".to_string(),
                version: 1,
                text: code.to_string(),
            },
        })
        .await;

    let params = HoverParams {
        text_document_position_params: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            position: Position {
                line: 4,
                character: 35,
            },
        },
        work_done_progress_params: Default::default(),
    };

    let result = service.inner().hover(params).await.unwrap();
    assert!(result.is_some());

    if let Some(hover) = result
        && let HoverContents::Markup(markup) = hover.contents
    {
        // Preceding comment should be ignored because there's a same-line comment
        assert!(!markup.value.contains("Preceding comment"));
        assert!(markup.value.contains("Following comment"));
    }

    let _ = fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_hover_local_var_comment() {
    let (service, _) = LspService::new(|client| {
        GameScriptLanguageServer::new(client, None, vec![], "test-version")
    });

    let temp_dir = std::env::current_dir()
        .unwrap()
        .join("target")
        .join("test_hover_local_var_comment");
    if temp_dir.exists() {
        fs::remove_dir_all(&temp_dir).unwrap();
    }
    fs::create_dir_all(&temp_dir).unwrap();

    let file_path = temp_dir.join("Test.gs");
    let code =
        "class Test {\n  public void Run() {\n    int i; // index variable\n    i = 0;\n  }\n};";
    fs::write(&file_path, code).unwrap();
    let uri = Uri::from_file_path(&file_path).unwrap();

    service
        .inner()
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "game-script".to_string(),
                version: 1,
                text: code.to_string(),
            },
        })
        .await;

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
    assert!(result.is_some());

    if let Some(hover) = result
        && let HoverContents::Markup(markup) = hover.contents
    {
        assert!(markup.value.contains("int i"));
        assert!(markup.value.contains("index variable"));
    }

    let _ = fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_hover_method_comment() {
    let (service, _) = LspService::new(|client| {
        GameScriptLanguageServer::new(client, None, vec![], "test-version")
    });

    let temp_dir = std::env::current_dir()
        .unwrap()
        .join("target")
        .join("test_hover_method_comment");
    if temp_dir.exists() {
        fs::remove_dir_all(&temp_dir).unwrap();
    }
    fs::create_dir_all(&temp_dir).unwrap();

    let file_path = temp_dir.join("Test.gs");
    let code = "class Test {\n  public void Run() { // starts execution\n  }\n\n  public void Start() { Run(); }\n};";
    fs::write(&file_path, code).unwrap();
    let uri = Uri::from_file_path(&file_path).unwrap();

    service
        .inner()
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "game-script".to_string(),
                version: 1,
                text: code.to_string(),
            },
        })
        .await;

    let params = HoverParams {
        text_document_position_params: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            position: Position {
                line: 4,
                character: 25,
            },
        },
        work_done_progress_params: Default::default(),
    };

    let result = service.inner().hover(params).await.unwrap();
    assert!(result.is_some());

    if let Some(hover) = result
        && let HoverContents::Markup(markup) = hover.contents
    {
        assert!(markup.value.contains("void Test::Run()"));
        assert!(markup.value.contains("starts execution"));
    }

    let _ = fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_hover_block_comment_same_line() {
    let (service, _) = LspService::new(|client| {
        GameScriptLanguageServer::new(client, None, vec![], "test-version")
    });

    let temp_dir = std::env::current_dir()
        .unwrap()
        .join("target")
        .join("test_hover_block_comment_same_line");
    if temp_dir.exists() {
        fs::remove_dir_all(&temp_dir).unwrap();
    }
    fs::create_dir_all(&temp_dir).unwrap();

    let file_path = temp_dir.join("Test.gs");
    let code = "class Test {\n  int m_val; /* internal value */\n\n  public int GetVal() { return m_val; }\n};";
    fs::write(&file_path, code).unwrap();
    let uri = Uri::from_file_path(&file_path).unwrap();

    service
        .inner()
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "game-script".to_string(),
                version: 1,
                text: code.to_string(),
            },
        })
        .await;

    let params = HoverParams {
        text_document_position_params: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            position: Position {
                line: 3,
                character: 33,
            },
        },
        work_done_progress_params: Default::default(),
    };

    let result = service.inner().hover(params).await.unwrap();
    assert!(result.is_some());

    if let Some(hover) = result
        && let HoverContents::Markup(markup) = hover.contents
    {
        assert!(markup.value.contains("int Test::m_val"));
        assert!(markup.value.contains("internal value"));
    }

    let _ = fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_hover_param_comment() {
    let (service, _) = LspService::new(|client| {
        GameScriptLanguageServer::new(client, None, vec![], "test-version")
    });

    let temp_dir = std::env::current_dir()
        .unwrap()
        .join("target")
        .join("test_hover_param_comment");
    if temp_dir.exists() {
        fs::remove_dir_all(&temp_dir).unwrap();
    }
    fs::create_dir_all(&temp_dir).unwrap();

    let file_path = temp_dir.join("Test.gs");
    let code = "class Test {\n  public void SetValue(int val // value to set\n  ) {\n    val = 1;\n  }\n};";
    fs::write(&file_path, code).unwrap();
    let uri = Uri::from_file_path(&file_path).unwrap();

    service
        .inner()
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "game-script".to_string(),
                version: 1,
                text: code.to_string(),
            },
        })
        .await;

    let params = HoverParams {
        text_document_position_params: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            position: Position {
                line: 3,
                character: 5,
            },
        },
        work_done_progress_params: Default::default(),
    };

    let result = service.inner().hover(params).await.unwrap();
    assert!(result.is_some());

    if let Some(hover) = result
        && let HoverContents::Markup(markup) = hover.contents
    {
        assert!(markup.value.contains("parameter: int val"));
        assert!(markup.value.contains("value to set"));
    }

    let _ = fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_hover_param_no_preceding_comment() {
    let (service, _) = LspService::new(|client| {
        GameScriptLanguageServer::new(client, None, vec![], "test-version")
    });

    let temp_dir = std::env::current_dir()
        .unwrap()
        .join("target")
        .join("test_hover_param_no_preceding_comment");
    if temp_dir.exists() {
        fs::remove_dir_all(&temp_dir).unwrap();
    }
    fs::create_dir_all(&temp_dir).unwrap();

    let file_path = temp_dir.join("Test.gs");
    let code =
        "class Test {\n  // Method documentation\n  public void Run(int p) {\n    p = 1;\n  }\n};";
    fs::write(&file_path, code).unwrap();
    let uri = Uri::from_file_path(&file_path).unwrap();

    service
        .inner()
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "game-script".to_string(),
                version: 1,
                text: code.to_string(),
            },
        })
        .await;

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
    assert!(result.is_some());

    if let Some(hover) = result
        && let HoverContents::Markup(markup) = hover.contents
    {
        assert!(markup.value.contains("parameter: int p"));
        // Method documentation should NOT be included for parameters
        assert!(!markup.value.contains("Method documentation"));
    }

    let _ = fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_hover_multiline_parm_comment() {
    let (service, _) = LspService::new(|client| {
        GameScriptLanguageServer::new(client, None, vec![], "test-version")
    });

    let temp_dir = std::env::current_dir()
        .unwrap()
        .join("target")
        .join("test_hover_multiline_parm_comment");
    if temp_dir.exists() {
        fs::remove_dir_all(&temp_dir).unwrap();
    }
    fs::create_dir_all(&temp_dir).unwrap();

    let file_path = temp_dir.join("Test.gs");
    let code = r#"
class Test {
  /**
   * Parm: browser - The Browser control which is used to visualise this 
   *       GameplayMenu.
   * Parm: menuMode - One of the MENUMODE_* defines, indicating which mode we
   *       are switching to.
   */
  public void SetMenu(object browser, int menuMode) {
  }
};
"#;
    fs::write(&file_path, code).unwrap();
    let uri = Uri::from_file_path(&file_path).unwrap();

    service
        .inner()
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "game-script".to_string(),
                version: 1,
                text: code.to_string(),
            },
        })
        .await;

    let params = HoverParams {
        text_document_position_params: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            position: Position {
                line: 8,
                character: 15, // On 'SetMenu'
            },
        },
        work_done_progress_params: Default::default(),
    };

    let result = service.inner().hover(params).await.unwrap();
    assert!(result.is_some());

    if let Some(hover) = result
        && let HoverContents::Markup(markup) = hover.contents
    {
        assert!(markup.value.contains(
            "Parm: browser - The Browser control which is used to visualise this GameplayMenu."
        ));
        assert!(markup.value.contains("Parm: menuMode - One of the MENUMODE_* defines, indicating which mode we are switching to."));
    }

    let _ = fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_hover_multiline_desc_comment() {
    let (service, _) = LspService::new(|client| {
        GameScriptLanguageServer::new(client, None, vec![], "test-version")
    });

    let temp_dir = std::env::current_dir()
        .unwrap()
        .join("target")
        .join("test_hover_multiline_desc_comment");
    if temp_dir.exists() {
        fs::remove_dir_all(&temp_dir).unwrap();
    }
    fs::create_dir_all(&temp_dir).unwrap();

    let file_path = temp_dir.join("Test.gs");
    let code = r#"
class Test {
  /**
   * Desc: Called whenever the user requests a "go back" action. If the user has
   *       navigated to some form of "sub menu", this should return them to the
   *       main level of this gameplay menu. If at the main level, this should
   *       close the gameplay menu and return to the game main menu.
   */
  public void GoBack() {
  }
};
"#;
    fs::write(&file_path, code).unwrap();
    let uri = Uri::from_file_path(&file_path).unwrap();

    service
        .inner()
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "game-script".to_string(),
                version: 1,
                text: code.to_string(),
            },
        })
        .await;

    let params = HoverParams {
        text_document_position_params: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            position: Position {
                line: 8,
                character: 15, // On 'GoBack'
            },
        },
        work_done_progress_params: Default::default(),
    };

    let result = service.inner().hover(params).await.unwrap();
    assert!(result.is_some());

    if let Some(hover) = result
        && let HoverContents::Markup(markup) = hover.contents
    {
        assert!(markup.value.contains("Desc: Called whenever the user requests a \"go back\" action. If the user has navigated to some form of \"sub menu\", this should return them to the main level of this gameplay menu. If at the main level, this should close the gameplay menu and return to the game main menu."));
    }

    let _ = fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_hover_multiline_other_tags() {
    let (service, _) = LspService::new(|client| {
        GameScriptLanguageServer::new(client, None, vec![], "test-version")
    });

    let temp_dir = std::env::current_dir()
        .unwrap()
        .join("target")
        .join("test_hover_multiline_other_tags");
    if temp_dir.exists() {
        fs::remove_dir_all(&temp_dir).unwrap();
    }
    fs::create_dir_all(&temp_dir).unwrap();

    let file_path = temp_dir.join("Test.gs");
    let code = r#"
class Test {
  /**
   * Retn: True if the operation was successful, false
   *       otherwise.
   * File: some_file.gs - The source file containing this
   *       class definition.
   * See Also: OtherClass.SomeMethod() for more
   *           information.
   */
  public bool DoSomething() {
    return true;
  }
};
"#;
    fs::write(&file_path, code).unwrap();
    let uri = Uri::from_file_path(&file_path).unwrap();

    service
        .inner()
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "game-script".to_string(),
                version: 1,
                text: code.to_string(),
            },
        })
        .await;

    let params = HoverParams {
        text_document_position_params: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            position: Position {
                line: 10,
                character: 15, // On 'DoSomething'
            },
        },
        work_done_progress_params: Default::default(),
    };

    let result = service.inner().hover(params).await.unwrap();
    assert!(result.is_some());

    if let Some(hover) = result
        && let HoverContents::Markup(markup) = hover.contents
    {
        assert!(
            markup
                .value
                .contains("Retn: True if the operation was successful, false otherwise.")
        );
        assert!(
            markup
                .value
                .contains("File: some_file.gs - The source file containing this class definition.")
        );
        assert!(
            markup
                .value
                .contains("See Also: OtherClass.SomeMethod() for more information.")
        );
    }

    let _ = fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_hover_documentation_at_top() {
    let (service, _) = LspService::new(|client| {
        GameScriptLanguageServer::new(client, None, vec![], "test-version")
    });

    let temp_dir = std::env::current_dir()
        .unwrap()
        .join("target")
        .join("test_hover_documentation_at_top");
    if temp_dir.exists() {
        let _ = fs::remove_dir_all(&temp_dir);
    }
    fs::create_dir_all(&temp_dir).unwrap();

    let file_path = temp_dir.join("Test.gs");
    let code = r#"
class Test {
  /**
   * This is the documentation.
   */
  public void MyMethod() {
  }

  public void CallMethod() {
    MyMethod();
  }
};
"#;
    fs::write(&file_path, code).unwrap();
    let uri = Uri::from_file_path(&file_path).unwrap();

    // Open file
    service
        .inner()
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "game-script".to_string(),
                version: 1,
                text: code.to_string(),
            },
        })
        .await;

    // Hover over 'MyMethod' in 'CallMethod'
    let params = HoverParams {
        text_document_position_params: TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            position: Position {
                line: 9,
                character: 6,
            },
        },
        work_done_progress_params: Default::default(),
    };

    let result = service.inner().hover(params).await.unwrap();
    assert!(result.is_some(), "Hover should return a result");

    if let Some(hover) = result {
        if let HoverContents::Markup(markup) = hover.contents {
            // Expected order:
            // This is the documentation.
            // ---
            // public void MyMethod()

            assert!(
                markup.value.contains("This is the documentation"),
                "Hover should contain documentation"
            );
            assert!(
                markup.value.contains("public void Test::MyMethod()"),
                "Hover should contain signature"
            );

            // Check if documentation is BEFORE the signature
            let doc_pos = markup.value.find("This is the documentation").unwrap();
            let sig_pos = markup.value.find("public void Test::MyMethod()").unwrap();

            assert!(
                doc_pos < sig_pos,
                "Documentation should be at the top. doc_pos: {}, sig_pos: {}\nContent: {}",
                doc_pos,
                sig_pos,
                markup.value
            );
        } else {
            panic!("Expected markup contents");
        }
    }

    let _ = fs::remove_dir_all(&temp_dir);
}
