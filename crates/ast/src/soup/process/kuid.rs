use crate::soup::Kuid;
use gs_parser::soup::grammar::Rule;
use gs_util::range::pair_to_range;
use pest::iterators::Pair;

pub fn process_kuid(pair: Pair<Rule>) -> Kuid {
    let range = pair_to_range(&pair);
    let mut user_id = 0;
    let mut content_id = 0;
    let mut version = None;

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::kuid_user_id => user_id = inner.as_str().parse().unwrap(),
            Rule::kuid_content_id => content_id = inner.as_str().parse().unwrap(),
            Rule::kuid_version_number => version = Some(inner.as_str().parse().unwrap()),
            _ => {}
        }
    }

    Kuid {
        user_id,
        content_id,
        version,
        range,
    }
}
