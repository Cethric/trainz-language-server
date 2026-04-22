use crate::acs_text::util::is_in_range;
use trainz_ast::Position;
use trainz_ast::acs_text::{KeyValuePair, Value};

pub fn find_kv_at_recursive<'a>(
    kvs: &'a [KeyValuePair],
    pos: Position,
) -> Option<&'a KeyValuePair> {
    let mut best_kv: Option<&KeyValuePair> = None;
    for kv in kvs {
        if is_in_range(pos, &kv.range) {
            // If we found a container, we might want to recurse inside it
            if let Some(value) = &kv.value {
                if let Value::Container(inner, _, _) = value {
                    if is_in_range(pos, &value.range()) {
                        if let Some(inner_kv) = find_kv_at_recursive(inner, pos) {
                            return Some(inner_kv);
                        }
                    }
                }
            }

            // Otherwise, keep track of the best match (most specific)
            if let Some(best) = best_kv {
                if kv.range.start.line > best.range.start.line {
                    best_kv = Some(kv);
                }
            } else {
                best_kv = Some(kv);
            }
        }
    }
    best_kv
}
