use criterion::{Criterion, criterion_group, criterion_main};
use std::path::PathBuf;
use tokio::runtime::Runtime;
use tower_lsp_server::LspService;
use tower_lsp_server::ls_types::ProgressToken;
use trainz_language_server::process::gs::ProcessGS;
use trainz_language_server::state::GameScriptLanguageServer;

async fn bench_process_gs_file(server: &GameScriptLanguageServer, path: PathBuf, content: &str) {
    let workspace_folders = vec![];

    let progress = server
        .client
        .progress(ProgressToken::String("bench".to_string()), "Benchmarking")
        .with_percentage(0)
        .begin()
        .await;

    server
        .process_gs_file(&path, content, &workspace_folders, false, &progress)
        .await;

    progress.finish().await;
}

fn criterion_benchmark(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let (service, _socket) = LspService::new(|client| {
        GameScriptLanguageServer::new(client, None, vec![], "test", None, None)
    });
    let server = service.inner();

    // In Cargo, benches run with CWD set to the crate root.
    // The test_programs are in this crate's test_programs folder.
    let mut path = std::env::current_dir().unwrap();
    path.push("test_programs");
    path.push("hello_world.gs");

    let content = std::fs::read_to_string(&path).expect("Could not read test file");

    c.bench_function("process_gs_file hello_world", |b| {
        b.to_async(&rt)
            .iter(|| bench_process_gs_file(server, path.clone(), &content));
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
