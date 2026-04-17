use crate::util::add_folding_range_a;
use rayon::iter::IntoParallelRefIterator;
use rayon::prelude::*;
use tower_lsp_server::ls_types::FoldingRange;
use trainz_ast::acs_text::Value;
use trainz_ast::acs_text::base::AcsText;

#[cfg(test)]
mod tests;

#[tracing::instrument(skip(acs_text))]
pub fn acs_text_folding_range(acs_text: &AcsText) -> Vec<FoldingRange> {
    acs_text
        .key_value_pairs
        .par_iter()
        .filter_map(|pair| pair.value.as_ref().map(collect_key_value_folding_ranges))
        .flatten()
        .collect::<Vec<FoldingRange>>()
}

#[tracing::instrument(skip(value))]
fn collect_key_value_folding_ranges(value: &Value) -> Vec<FoldingRange> {
    if let Value::Container(pairs, _, full_range) = value {
        let range = *full_range;
        let mut folding = pairs
            .par_iter()
            .filter_map(|p| p.value.as_ref().map(collect_key_value_folding_ranges))
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
