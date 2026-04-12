use crate::range::pair_to_range;
use pest::RuleType;
use pest::iterators::{Pair, Pairs};
use tower_lsp_server::ls_types::Position;

#[tracing::instrument]
pub fn find_at_position<Rule: RuleType>(
    pairs: Pairs<Rule>,
    position: Position,
) -> Option<Pair<Rule>> {
    for pair in pairs {
        let range = pair_to_range(&pair);

        if position >= range.start && position <= range.end {
            if let Some(inner) = find_at_position(pair.clone().into_inner(), position) {
                return Some(inner);
            }
            return Some(pair);
        }
    }
    None
}
