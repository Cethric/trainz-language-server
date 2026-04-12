use crate::gs::Include;
use pest::iterators::Pair;
use trainz_common::range::pair_to_range;
use trainz_parser::gs::grammar::Rule;

#[tracing::instrument]
pub fn process_include(include_rule: Pair<Rule>) -> Option<Include> {
    let range = pair_to_range(&include_rule);
    let mut include = Include {
        path: None,
        path_range: None,
        name: String::from(""),
        range,
        keyword_include_range: range,
    };

    let inner = include_rule.into_inner();
    for pair in inner {
        let rule = pair.as_rule();

        match rule {
            Rule::include_path => {
                include.path_range = Some(pair_to_range(&pair));
                let path = pair.as_str();

                let include_path = path
                    .strip_prefix('"')
                    .and_then(|s| s.strip_suffix('"'))
                    .unwrap_or("");

                include.name = include_path.to_string();
            }
            Rule::keyword_include => include.keyword_include_range = pair_to_range(&pair),
            _ => {}
        }
    }

    Some(include)
}
