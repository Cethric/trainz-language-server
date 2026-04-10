use criterion::{Criterion, criterion_group, criterion_main};
use pest::Parser;
use std::hint::black_box;
use trainz_ast::gs::process::process_trainz_ast;
use trainz_parser::gs::grammar::{GameScriptParser, Rule};

fn gs_benchmark(c: &mut Criterion) {
    let gs_code = r#"
        include "library.gs"
        include "util.gs"

        class MyController isclass MyBase
        {
            public void Init(string n, int i)
            {
                Print("Initialized ");
            }
        };

        class Utility
        {
            static public int Max(int a, int b)
            {
                if (a > b) { return a; }
                return b;
            }
        };
    "#
    .repeat(10);

    c.bench_function("process_trainz_ast", |b| {
        b.iter(|| {
            let pairs = GameScriptParser::parse(Rule::program, black_box(&gs_code)).unwrap();
            process_trainz_ast(black_box(pairs), black_box(&gs_code))
        })
    });

    c.bench_function("parse_gs_only", |b| {
        b.iter(|| GameScriptParser::parse(Rule::program, black_box(&gs_code)).unwrap())
    });
}

criterion_group!(benches, gs_benchmark);
criterion_main!(benches);
