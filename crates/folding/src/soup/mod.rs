use crate::util::add_folding_range;
use gs_ast::soup::{Soup, Value};
use tower_lsp_server::ls_types::FoldingRange;

#[cfg(test)]
mod tests;

pub fn soup_folding_range(soup: &Soup) -> Vec<FoldingRange> {
    let mut result = vec![];

    for pair in &soup.key_value_pairs {
        if let Some(value) = &pair.value {
            collect_value_folding_ranges(value, &mut result);
        }
    }

    result
}

fn collect_value_folding_ranges(value: &Value, result: &mut Vec<FoldingRange>) {
    match value {
        Value::Container(pairs, range) => {
            add_folding_range(
                *range,
                result,
                Some(format!("{{ {} properties }}", pairs.len())),
            );
            for pair in pairs {
                if let Some(val) = &pair.value {
                    collect_value_folding_ranges(val, result);
                }
            }
        }
        _ => {}
    }
}
