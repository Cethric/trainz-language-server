use criterion::{Criterion, criterion_group, criterion_main};
use pest::Parser;
use std::hint::black_box;
use trainz_ast::soup::process::process_soup_ast;
use trainz_parser::soup::grammar::{AuranConfigSoupParser, Rule};

fn soup_benchmark(c: &mut Criterion) {
    let large_soup = r#"
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

    c.bench_function("process_soup_ast", |b| {
        b.iter(|| {
            let pairs = AuranConfigSoupParser::parse(Rule::soup, black_box(&large_soup)).unwrap();
            process_soup_ast(black_box(pairs), black_box(&large_soup))
        })
    });

    c.bench_function("parse_soup_only", |b| {
        b.iter(|| AuranConfigSoupParser::parse(Rule::soup, black_box(&large_soup)).unwrap())
    });
}

criterion_group!(benches, soup_benchmark);
criterion_main!(benches);
