use rayon::prelude::*;
use tower_lsp_server::ls_types::{DocumentSymbol, SymbolKind};
use trainz_ast::soup::Value;
use trainz_ast::soup::base::Soup;
use trainz_ast::soup::key_value_pair::KeyValuePair;
use trainz_common::range::clamp_range;
use trainz_soup_validators::{ArrayElementType, ContainerValidator, Validators};

#[allow(deprecated)]
#[tracing::instrument]
pub fn soup_symboliser(soup: &Soup, validators: Option<&Validators>) -> Vec<DocumentSymbol> {
    let mut symbols = vec![];

    let kind_kv = soup
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

    for kv in &soup.key_value_pairs {
        symbols.push(process_key_value_symbol(kv, validator, validators));
    }

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
        for inner_kv in kv_pairs {
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
                                    vs.containers
                                        .par_iter()
                                        .find_first(|v| v.container_name.eq_ignore_ascii_case(tn))
                                })
                            })
                        })
                });

            children.push(process_key_value_symbol(
                inner_kv,
                inner_validator,
                all_validators,
            ));
        }
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
    use trainz_ast::soup::process::process_soup_ast;
    use trainz_parser::soup::parse_soup;
    use trainz_soup_validators::{ContainerRule, ContainerValidator, Validators};

    #[allow(deprecated)]
    #[test]
    fn test_soup_symbol_deprecation() {
        let code = r#"
        kind "test-container"
        obsolete-key "value"
        "#;
        let pairs = parse_soup(code).unwrap();
        let soup = process_soup_ast(pairs, code);

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

        let symbols = soup_symboliser(&soup, Some(&validators));
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
    fn test_soup_multiline_string_symbol() {
        let code = r#"
        description "This is a
        multi-line
        string"
        "#;
        let pairs = parse_soup(code).unwrap();
        let soup = process_soup_ast(pairs, code);
        let symbols = soup_symboliser(&soup, None);

        let desc_symbol = symbols
            .par_iter()
            .find_first(|s| s.name == "description")
            .unwrap();
        // The symbol range should cover all lines of the multi-line string.
        assert_eq!(desc_symbol.range.start.line, 1);
        assert_eq!(desc_symbol.range.end.line, 4);
    }
}
