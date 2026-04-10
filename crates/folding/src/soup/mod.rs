use crate::util::add_folding_range_a;
use rayon::iter::IntoParallelRefIterator;
use rayon::prelude::*;
use tower_lsp_server::ls_types::FoldingRange;
use trainz_ast::soup::Value;
use trainz_ast::soup::soup::Soup;

#[cfg(test)]
mod tests;

pub fn soup_folding_range(soup: &Soup) -> Vec<FoldingRange> {
    soup.key_value_pairs
        .par_iter()
        .filter_map(|pair| {
            if let Some(value) = &pair.value {
                Some(collect_key_value_folding_ranges(value))
            } else {
                None
            }
        })
        .flatten()
        .collect::<Vec<FoldingRange>>()
}

fn collect_key_value_folding_ranges(value: &Value) -> Vec<FoldingRange> {
    if let Value::Container(pairs, _, full_range) = value {
        let range = *full_range;
        let mut folding = pairs
            .par_iter()
            .filter_map(|p| {
                if let Some(val) = &p.value {
                    Some(collect_key_value_folding_ranges(val))
                } else {
                    None
                }
            })
            .flatten()
            .collect::<Vec<FoldingRange>>();
        if let Some(range) =
            add_folding_range_a(range, Some(format!("{{ {} properties }}", pairs.len())))
        {
            folding.push(range);
        }
        folding.sort_by(|a, b| {
            a.start_line
                .cmp(&b.start_line)
                .then(a.start_character.cmp(&b.start_character))
        });

        folding
    } else {
        vec![]
    }
}
