use criterion::{Criterion, criterion_group, criterion_main};
use std::path::PathBuf;
use tokio::runtime::Runtime;
use tower_lsp_server::LspService;
use tower_lsp_server::ls_types::ProgressToken;
use trainz_language_server::process::soup::ProcessSoup;
use trainz_language_server::state::GameScriptLanguageServer;

async fn bench_process_soup_file(server: &GameScriptLanguageServer, path: PathBuf, content: &str) {
    let progress = server
        .client
        .progress(ProgressToken::String("bench".to_string()), "Benchmarking")
        .with_percentage(0)
        .begin()
        .await;

    server
        .process_soup_file(&path, content, false, &progress)
        .await;

    progress.finish().await;
}

fn criterion_benchmark(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let (service, _socket) =
        LspService::new(|client| GameScriptLanguageServer::new(client, None, vec![], "test"));
    let server = service.inner();

    let mut path = std::env::current_dir().unwrap();
    path.push("test_programs");
    path.push("sample.soup");

    let content = std::fs::read_to_string(&path).expect("Could not read test file");

    c.bench_function("process_soup_file sample", |b| {
        b.to_async(&rt)
            .iter(|| bench_process_soup_file(server, path.clone(), &content));
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
