use criterion::{Criterion, criterion_group, criterion_main};
use pest::Parser;
use std::hint::black_box;
use trainz_ast::acs_text::process::process_acs_text_ast;
use trainz_parser::acs_text::grammar::{AuranConfigAcsTextParser, Rule};

fn acs_text_benchmark(c: &mut Criterion) {
    let large_acs_text = r#"
        kuid <kuid:123:456>
        username "My Test Object"
        kind "scenery"
        trainz-build 4.5

        mesh-table
        {
            default
            {
                mesh "default.trainzmesh"
                auto-create 1
            }
            lod1
            {
                mesh "lod1.trainzmesh"
                auto-create 1
            }
        }

        extensions
        {
            my-ext-1
            {
                foo 1
                bar 2.5
                baz "hello"
            }
        }

        thumbnails
        {
            0
            {
                image "thumb.jpg"
                width 240
                height 180
            }
        }

        description "This is a long description to test string processing. It contains multiple words and should be a bit larger than other strings."

        category-class "AS"
        category-region "AU"
        category-era "2010s"

        kuid-table
        {
            0 <kuid:123:1>
            1 <kuid:123:2>
            2 <kuid:123:3>
            3 <kuid:123:4>
            4 <kuid:123:5>
        }
    "#.repeat(10); // Make it 10x larger for better measurement

    c.bench_function("process_acs_text_ast", |b| {
        b.iter(|| {
            let pairs = AuranConfigAcsTextParser::parse(Rule::acs_text, black_box(&large_acs_text))
                .unwrap();
            process_acs_text_ast(black_box(pairs), black_box(&large_acs_text))
        })
    });

    c.bench_function("parse_acs_text_only", |b| {
        b.iter(|| {
            AuranConfigAcsTextParser::parse(Rule::acs_text, black_box(&large_acs_text)).unwrap()
        })
    });
}

criterion_group!(benches, acs_text_benchmark);
criterion_main!(benches);
