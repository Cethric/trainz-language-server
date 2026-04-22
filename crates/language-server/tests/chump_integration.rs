// use std::fs;
// use tower_lsp_server::ls_types::*;
// use tower_lsp_server::LanguageServer;
// use trainz_language_server::state::{TrainzLanguageServer, ParsedFileType};
// use trainz_tdx::TdxValue;
//
// #[tokio::test]
// async fn test_chump_indexing() {
//     let (service, _) = tower_lsp_server::LspService::new(|client| {
//         TrainzLanguageServer::new(client, None, vec![], "test-version", None, None)
//     });
//
//     let temp_dir = std::env::current_dir()
//         .unwrap()
//         .join("target")
//         .join("test_chump_indexing");
//     if temp_dir.exists() {
//         fs::remove_dir_all(&temp_dir).unwrap();
//     }
//     fs::create_dir_all(&temp_dir).unwrap();
//
//     // Create a config.txt to define the project
//     let config_path = temp_dir.join("config.txt");
//     fs::write(&config_path, "kuid <kuid:1:1>\nkind \"asset\"").unwrap();
//
//     // Create a .chp file
//     let mut chp_data = vec![0xdb, 0xa2, 0xeb, 0xce];
//     // Filler
//     chp_data.extend_from_slice(&[0; 50]);
//     // ACS$ magic (skipped by TdxReader::read_entry)
//     chp_data.extend_from_slice(b"ACS$");
//     chp_data.extend_from_slice(&[0; 8]); // version and unk
//     // Size
//     let entry_size: u32 = 1 + 4 + 1 + 5; // name_len + name + kind + data
//     chp_data.extend_from_slice(&entry_size.to_le_bytes());
//     // Entry
//     chp_data.push(4); // name_len
//     chp_data.extend_from_slice(b"test");
//     chp_data.push(0x03); // String
//     chp_data.extend_from_slice(b"hello");
//
//     let chp_path = temp_dir.join("test.chp");
//     fs::write(&chp_path, &chp_data).unwrap();
//
//     // Initialize server with workspace folder
//     service
//         .inner()
//         .initialize(InitializeParams {
//             workspace_folders: Some(vec![WorkspaceFolder {
//                 uri: Uri::from_file_path(&temp_dir).unwrap(),
//                 name: "test".to_string(),
//             }]),
//             ..Default::default()
//         })
//         .await
//         .unwrap();
//
//     service.inner().initialized(InitializedParams {}).await;
//
//     // Wait for indexing to complete
//     // In tests, we might need to wait a bit or call discover_projects explicitly if possible.
//     // However, initialize usually calls discover_projects.
//
//     // Check if the file was parsed
//     let path_str = chp_path.to_string_lossy().to_string();
//
//     // We might need to give it a moment to index
//     let mut found = false;
//     for _ in 0..20 {
//         if let Some(file) = service.inner().parsed_files.get(&path_str) {
//             if let ParsedFileType::AcsBinary(results) = &file.parsed {
//                 if let Some((_, TdxValue::String(s))) = results.iter().find(|(k, _)| k == "test") {
//                     if s == "hello" {
//                         found = true;
//                         break;
//                     }
//                 }
//             }
//         }
//         tokio::time::sleep(std::time::Duration::from_millis(500)).await;
//     }
//
//     assert!(found, "Chump file should be indexed and contain 'test' entry");
//
//     let _ = fs::remove_dir_all(&temp_dir);
// }
