use crate::acs_text::find_kv_at_recursive;
use crate::acs_text::util::is_in_range;
use crate::acs_text::value::find_hover_in_value;
use std::path::Path;
use tower_lsp_server::ls_types::{Hover, HoverParams};
use trainz_ast::acs_text::{AcsText, Value};

#[tracing::instrument(skip(acs_text, params, base_path, script_resolver))]
pub fn acs_text_hover(
    acs_text: &AcsText,
    params: HoverParams,
    base_path: Option<&Path>,
    script_resolver: Option<&dyn trainz_definition::acs_text::definitions::ScriptResolver>,
) -> Option<Hover> {
    let position = params.text_document_position_params.position;

    let kv = find_kv_at_recursive::find_kv_at_recursive(&acs_text.key_value_pairs, position);

    if let Some(kv) = kv {
        if let Some(value) = &kv.value
            && is_in_range(position, &value.range())
        {
            // Find trainz-build version in current scope or parent
            // (Old logic for trainz-build is still useful)
            let trainz_build_version = acs_text
                .key_value_pairs
                .iter()
                .find(|kv| kv.key.eq_ignore_ascii_case("trainz-build"))
                .and_then(|kv| match &kv.value {
                    Some(Value::Numeric(trainz_ast::acs_text::NumericValue::Float(f), _)) => {
                        Some(*f)
                    }
                    Some(Value::Numeric(trainz_ast::acs_text::NumericValue::Int(i), _)) => {
                        Some(*i as f64)
                    }
                    Some(Value::String(s, _)) => s.parse::<f64>().ok(),
                    _ => None,
                });

            // Value hover (KUID, Script, Image, etc.)

            let hover = find_hover_in_value(
                value,
                position,
                base_path,
                script_resolver,
                trainz_build_version,
            );

            if hover.is_some() {
                return hover;
            }

            return None;
        }
    }

    None
}
