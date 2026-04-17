pub mod kuid;
pub mod value;

use crate::acs_text::base::AcsText;
use crate::acs_text::key_value_pair::KeyValuePair;
use pest::iterators::{Pair, Pairs};
use tower_lsp_server::ls_types::Range;
use trainz_common::range::{pair_to_range, pos_to_range};
use trainz_parser::acs_text::grammar::Rule;
use value::process_value;

#[tracing::instrument(skip(pairs))]
pub fn process_acs_text_ast(pairs: Pairs<Rule>, src: &str) -> AcsText {
    let mut key_value_pairs = vec![];
    let mut root_range = Range::default();

    fn traverse(pairs: Pairs<Rule>, key_value_pairs: &mut Vec<KeyValuePair>) {
        for pair in pairs {
            match pair.as_rule() {
                Rule::key_value_pair => {
                    key_value_pairs.push(process_key_value_pair(pair));
                }
                _ => traverse(pair.into_inner(), key_value_pairs),
            }
        }
    }

    traverse(pairs.clone(), &mut key_value_pairs);

    if let Some(first_pair) = pairs.clone().next() {
        let first_pos = first_pair.as_span().start_pos();
        let last_pos = pairs.clone().last().unwrap().as_span().end_pos();
        root_range = pos_to_range(&first_pos, &last_pos);
    }

    AcsText {
        key_value_pairs,
        range: root_range,
        src: src.to_string(),
    }
}

#[tracing::instrument(skip(pair))]
pub fn process_key_value_pair(pair: Pair<Rule>) -> KeyValuePair {
    let range = pair_to_range(&pair);
    let mut inner = pair.into_inner();
    let key_pair = inner.next().unwrap();
    let key_range = pair_to_range(&key_pair);
    let key = key_pair.as_str().to_string();
    let value = inner.next().map(process_value);

    KeyValuePair {
        key,
        key_range,
        value,
        range,
    }
}
