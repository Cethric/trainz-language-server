use crate::acs_text::util::is_in_range;
use std::path::Path;
use tower_lsp_server::ls_types::Hover;
use trainz_ast::Position;
use trainz_ast::acs_text::{KeyValuePair, Value};

/// Compatibility wrapper for value hover recursion.
pub fn acs_text_hover_recursive_wrapper(
    kvs: &[KeyValuePair],
    position: Position,
    base_path: Option<&Path>,
    script_resolver: Option<&dyn trainz_definition::acs_text::definitions::ScriptResolver>,
    trainz_build_version: Option<f64>,
) -> Option<Hover> {
    // Minimal implementation: just check simple values at this level.
    for kv in kvs {
        if let Some(value) = &kv.value {
            if is_in_range(position, &value.range()) {
                if let Value::Container(inner, _, _) = value {
                    // Recurse
                    return acs_text_hover_recursive_wrapper(
                        inner,
                        position,
                        base_path,
                        script_resolver,
                        trainz_build_version,
                    );
                }
            }
        }
    }
    None
}
