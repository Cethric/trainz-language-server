use rayon::prelude::*;
use tower_lsp_server::ls_types::{DocumentSymbol, SymbolKind};
use trainz_acs_text_validators::{ArrayElementType, ContainerValidator, Validators};
use trainz_ast::acs_text::Value;
use trainz_ast::acs_text::base::AcsText;
use trainz_ast::acs_text::key_value_pair::KeyValuePair;
use trainz_common::range::clamp_range;

#[allow(deprecated)]
#[tracing::instrument]
pub fn acs_text_symboliser(
    acs_text: &AcsText,
    validators: Option<&Validators>,
) -> Vec<DocumentSymbol> {
    let kind_kv = acs_text
        .key_value_pairs
        .par_iter()
        .find_first(|kv| kv.key.eq_ignore_ascii_case("kind"));
    let validator = kind_kv.and_then(|kv| match &kv.value {
        Some(Value::String(s, _)) | Some(Value::Variable(s, _)) => validators.and_then(|vs| {
            vs.containers
                .par_iter()
                .find_first(|v| v.container_name.eq_ignore_ascii_case(s))
        }),
        _ => None,
    });

    let symbols: Vec<DocumentSymbol> = acs_text
        .key_value_pairs
        .par_iter()
        .map(|kv| process_key_value_symbol(kv, validator, validators))
        .collect();

    symbols
}

#[allow(deprecated)]
#[tracing::instrument]
fn process_key_value_symbol(
    kv: &KeyValuePair,
    validator: Option<&ContainerValidator>,
    all_validators: Option<&Validators>,
) -> DocumentSymbol {
    let mut children = vec![];

    let rule = validator.and_then(|v| {
        v.rules
            .par_iter()
            .find_first(|r| r.key.eq_ignore_ascii_case(&kv.key))
            .or_else(|| {
                v.sub_possibilities
                    .par_iter()
                    .find_first(|r| r.key.eq_ignore_ascii_case(&kv.key))
            })
    });

    let is_deprecated = rule.and_then(|r| r.obsolete_tag).unwrap_or(false);

    if let Some(value) = &kv.value
        && let Value::Container(kv_pairs, _, _) = value
    {
        children = kv_pairs
            .par_iter()
            .map(|inner_kv| {
                let inner_validator = rule
                    .and_then(|r| r.type_name.as_ref())
                    .and_then(|type_name| {
                        all_validators.and_then(|vs| {
                            vs.containers
                                .par_iter()
                                .find_first(|v| v.container_name.eq_ignore_ascii_case(type_name))
                        })
                    })
                    .or_else(|| {
                        validator
                            .and_then(|v| v.array_element.as_ref())
                            .and_then(|ae| {
                                let type_name = match ae {
                                    ArrayElementType::Array(s) => Some(s),
                                    ArrayElementType::Tuple(types) => inner_kv
                                        .key
                                        .parse::<usize>()
                                        .ok()
                                        .and_then(|idx| types.get(idx)),
                                };
                                type_name.and_then(|tn| {
                                    all_validators.and_then(|vs| {
                                        vs.containers.par_iter().find_first(|v| {
                                            v.container_name.eq_ignore_ascii_case(tn)
                                        })
                                    })
                                })
                            })
                    });

                process_key_value_symbol(inner_kv, inner_validator, all_validators)
            })
            .collect();
    }

    DocumentSymbol {
        name: kv.key.clone(),
        detail: None,
        kind: if children.is_empty() {
            SymbolKind::PROPERTY
        } else {
            SymbolKind::NAMESPACE
        },
        tags: if is_deprecated {
            Some(vec![tower_lsp_server::ls_types::SymbolTag::DEPRECATED])
        } else {
            None
        },
        deprecated: if is_deprecated { Some(true) } else { None },
        range: kv.range,
        selection_range: clamp_range(&kv.range, kv.key_range),
        children: if children.is_empty() {
            None
        } else {
            Some(children)
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use trainz_acs_text_validators::{ContainerRule, ContainerValidator, Validators};
    use trainz_ast::acs_text::process::process_acs_text_ast;
    use trainz_parser::acs_text::parse_acs_text;

    #[allow(deprecated)]
    #[test]
    fn test_acs_text_symbol_deprecation() {
        let code = r#"
        kind "test-container"
        obsolete-key "value"
        "#;
        let pairs = parse_acs_text(code).unwrap();
        let acs_text = process_acs_text_ast(pairs, code);

        let mut validators = Validators::default();
        validators.containers.push(ContainerValidator {
            container_name: "test-container".to_string(),
            top_level: true,
            rules: vec![ContainerRule {
                key: "obsolete-key".to_string(),
                obsolete_tag: Some(true),
                ..Default::default()
            }],
            ..Default::default()
        });

        let symbols = acs_text_symboliser(&acs_text, Some(&validators));
        let obsolete_symbol = symbols
            .par_iter()
            .find_first(|s| s.name == "obsolete-key")
            .unwrap();

        assert!(obsolete_symbol.deprecated.unwrap_or(false));
        assert!(
            obsolete_symbol
                .tags
                .as_ref()
                .unwrap()
                .contains(&tower_lsp_server::ls_types::SymbolTag::DEPRECATED)
        );
    }

    #[test]
    fn test_acs_text_multiline_string_symbol() {
        let code = r#"
        description "This is a
        multi-line
        string"
        "#;
        let pairs = parse_acs_text(code).unwrap();
        let acs_text = process_acs_text_ast(pairs, code);
        let symbols = acs_text_symboliser(&acs_text, None);

        let desc_symbol = symbols
            .par_iter()
            .find_first(|s| s.name == "description")
            .unwrap();
        // The symbol range should cover all lines of the multi-line string.
        assert_eq!(desc_symbol.range.start.line, 1);
        assert_eq!(desc_symbol.range.end.line, 4);
    }
}
