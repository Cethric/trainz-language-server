pub mod kuid;
pub mod value;

use crate::soup::{KeyValuePair, Soup};
use gs_parser::soup::grammar::Rule;
use gs_util::range::{pair_to_range, pos_to_range};
use pest::iterators::{Pair, Pairs};
use std::path::{Path, PathBuf};
use tower_lsp_server::ls_types::Range;
use value::process_value;

pub fn process_soup_ast(
    pairs: Pairs<Rule>,
    src: &str,
    _base_path: &Path,
    _workspace_folders: &Vec<PathBuf>,
    _search_paths: &Vec<PathBuf>,
) -> Soup {
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

    Soup {
        key_value_pairs,
        range: root_range,
        src: src.to_string(),
    }
}

pub fn process_key_value_pair(pair: Pair<Rule>) -> KeyValuePair {
    let mut range = pair_to_range(&pair);
    if range.end.line > 0 {
        range.end.line = range.end.line - 1;
    }
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
