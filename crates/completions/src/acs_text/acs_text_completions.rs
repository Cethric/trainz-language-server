use rayon::prelude::*;
use rayon::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;
use tower_lsp_server::ls_types::{
    CompletionItem, CompletionItemKind, CompletionItemLabelDetails, CompletionItemTag,
    CompletionParams, CompletionTextEdit, Documentation, InsertTextFormat, InsertTextMode,
    MarkupContent, MarkupKind, TextEdit,
};
use tracing::debug;
use trainz_acs_text_validators::text_util::{get_kind_from_text, get_trainz_build_from_text};
use trainz_acs_text_validators::validation_graph::node::RuleNodeKind;
use trainz_acs_text_validators::validation_graph::value::RuleNodeValue;
use trainz_acs_text_validators::{RuleNode, RulesRoot};
use trainz_ast::acs_text::{AcsText, KeyValuePair};

#[tracing::instrument(skip(acs_text, graph, params, _asset_cache_path))]
pub fn acs_text_completions(
    acs_text: &AcsText,
    graph: &RulesRoot,
    params: CompletionParams,
    _asset_cache_path: Option<&std::path::Path>,
) -> Vec<CompletionItem> {
    let container_path = acs_text.get_kvp_to_position(params.text_document_position.position);
    let last_item = container_path.last();

    if let Some((kind, _, _)) = get_kind_from_text(acs_text)
        && let Some((found, chain)) = graph.get_rule_from_path(&kind, &container_path)
    {
        let trainz_build = get_trainz_build_from_text(acs_text);

        if found && let Some(rule) = chain.last() {
            build_completion_items(last_item, rule, trainz_build)
        } else if !found {
            chain
                .par_iter()
                .map(|node| build_completion_items(last_item, node, trainz_build))
                .flatten()
                .collect()
        } else {
            vec![]
        }
    } else {
        vec![]
    }
}

#[tracing::instrument(skip(last_item, node, trainz_build))]
fn build_completion_items(
    last_item: Option<&KeyValuePair>,
    node: &Arc<RuleNode>,
    trainz_build: f64,
) -> Vec<CompletionItem> {
    if let Some(kind) = node.kind() {
        match kind {
            RuleNodeKind::Value(value) => {
                build_value_completion_item(last_item, node, value, &trainz_build)
            }
            RuleNodeKind::Element(_) => {
                vec![]
            }
            RuleNodeKind::Structure(structure) => {
                if !structure.array_elements().is_empty() || structure.tag_array().is_some() {
                    let mut result = vec![];
                    if let Some(tag_array) = structure.tag_array()
                        && let Some(tag_array) = tag_array.upgrade()
                    {
                        result.par_extend(build_tag_array_completion_item(
                            &tag_array,
                            last_item.map_or(Some(node.name()), |n| Some(n.key.clone())),
                            &trainz_build,
                        ))
                    }

                    result.par_extend(
                        structure
                            .array_elements()
                            .par_iter()
                            .filter_map(|element| {
                                element.upgrade().map(|element| {
                                    build_completion_items(last_item, &element, trainz_build)
                                })
                            })
                            .flatten(),
                    );

                    result
                } else {
                    structure
                        .possibilities()
                        .par_iter()
                        .filter_map(|(_, rule)| {
                            rule.upgrade()
                                .map(|rule| build_completion_items(last_item, &rule, trainz_build))
                        })
                        .flatten()
                        .collect()
                }
            }
        }
    } else {
        vec![]
    }
}

#[tracing::instrument(skip(node, trainz_build))]
fn build_completion_item(node: &Arc<RuleNode>, trainz_build: &f64) -> Vec<CompletionItem> {
    debug!("Building completion item {:?}", node);

    vec![CompletionItem {
        label: node.name(),
        label_details: Some(CompletionItemLabelDetails {
            detail: None,
            description: node.description(),
        }),
        kind: Some(CompletionItemKind::KEYWORD),
        detail: node.details(),
        documentation: node.documentation(trainz_build).map(|value| {
            let result = value.join("\n\n");
            Documentation::MarkupContent(MarkupContent {
                kind: MarkupKind::Markdown,
                value: result,
            })
        }),
        deprecated: Some(node.is_obsolete(trainz_build)),
        preselect: None,
        sort_text: Some(node.name()),
        filter_text: Some(node.name()),
        insert_text: node.kind().map(|kind| {
            format!(
                "{} {}",
                node.name(),
                if let RuleNodeKind::Value(value) = kind {
                    if let RuleNodeValue::String(_) = value {
                        "\"$1\""
                    } else if let RuleNodeValue::FilePath(_) = value {
                        "\"$1\""
                    } else {
                        "$1"
                    }
                } else {
                    "{ $1 }"
                }
            )
        }),
        insert_text_format: if node.kind().is_some() {
            Some(InsertTextFormat::SNIPPET)
        } else {
            None
        },
        insert_text_mode: if node.kind().is_some() {
            Some(InsertTextMode::AS_IS)
        } else {
            None
        },
        text_edit: None,
        additional_text_edits: None,
        command: None,
        commit_characters: None,
        data: None,
        tags: if node.is_obsolete(trainz_build) {
            Some(vec![CompletionItemTag::DEPRECATED])
        } else {
            None
        },
    }]
}

#[tracing::instrument(skip(node, value, trainz_build))]
fn build_value_completion_item(
    key_value_pair: Option<&KeyValuePair>,
    node: &Arc<RuleNode>,
    value: &RuleNodeValue,
    trainz_build: &f64,
) -> Vec<CompletionItem> {
    debug!("Building value completion items {:?}", node);
    #[tracing::instrument(skip(key_value_pair, node, options, trainz_build, is_string))]
    fn options_to_completion_item(
        key_value_pair: Option<&KeyValuePair>,
        node: &Arc<RuleNode>,
        options: &HashMap<String, Option<String>>,
        trainz_build: &f64,
        is_string: bool,
    ) -> Vec<CompletionItem> {
        options
            .par_iter()
            .map(|(label, description)| {
                let value_text = if is_string {
                    format!("\"{}\"", label.clone())
                } else {
                    label.clone()
                };

                CompletionItem {
                    label: label.clone(),
                    label_details: Some(CompletionItemLabelDetails {
                        detail: None,
                        description: node.description(),
                    }),
                    kind: Some(CompletionItemKind::ENUM_MEMBER),
                    detail: description.clone(),
                    documentation: description.as_ref().map(|description| {
                        Documentation::MarkupContent(MarkupContent {
                            kind: MarkupKind::Markdown,
                            value: description.clone(),
                        })
                    }),
                    deprecated: Some(node.is_obsolete(trainz_build)),
                    preselect: None,
                    sort_text: Some(label.clone()),
                    filter_text: Some(format!(
                        "{}-{}",
                        label.clone(),
                        description.clone().unwrap_or(String::from(""))
                    )),
                    insert_text: Some(value_text.clone()),
                    insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                    insert_text_mode: Some(InsertTextMode::AS_IS),
                    text_edit: if let Some(kvp) = &key_value_pair {
                        kvp.value.as_ref().map(|value| {
                            CompletionTextEdit::Edit(TextEdit {
                                range: value.range(),
                                new_text: value_text,
                            })
                        })
                    } else {
                        None
                    },
                    additional_text_edits: None,
                    command: None,
                    commit_characters: None,
                    data: None,
                    tags: if node.is_obsolete(trainz_build) {
                        Some(vec![CompletionItemTag::DEPRECATED])
                    } else {
                        None
                    },
                }
            })
            .collect::<Vec<CompletionItem>>()
    }

    #[tracing::instrument(skip(key_value_pair, node, value, trainz_build))]
    fn build_completion_items_for_value(
        key_value_pair: Option<&KeyValuePair>,
        node: &Arc<RuleNode>,
        value: &RuleNodeValue,
        trainz_build: &f64,
    ) -> Vec<CompletionItem> {
        match value {
            RuleNodeValue::String(string) => {
                let mut completions = vec![CompletionItem {
                    label: String::from("String"),
                    label_details: Some(CompletionItemLabelDetails {
                        detail: None,
                        description: node.description(),
                    }),
                    kind: Some(CompletionItemKind::VALUE),
                    detail: node.details(),
                    documentation: None,
                    deprecated: Some(node.is_obsolete(trainz_build)),
                    preselect: None,
                    sort_text: None,
                    filter_text: None,
                    insert_text: Some(String::from("\"$1\"")),
                    insert_text_format: Some(InsertTextFormat::SNIPPET),
                    insert_text_mode: Some(InsertTextMode::AS_IS),
                    text_edit: None,
                    additional_text_edits: None,
                    command: None,
                    commit_characters: None,
                    data: None,
                    tags: if node.is_obsolete(trainz_build) {
                        Some(vec![CompletionItemTag::DEPRECATED])
                    } else {
                        None
                    },
                }];

                if let Some(default) = string.default() {
                    completions.push(CompletionItem {
                        label: default.clone(),
                        label_details: Some(CompletionItemLabelDetails {
                            detail: None,
                            description: node.description(),
                        }),
                        kind: Some(CompletionItemKind::VALUE),
                        detail: node.details(),
                        documentation: None,
                        deprecated: Some(node.is_obsolete(trainz_build)),
                        preselect: None,
                        sort_text: None,
                        filter_text: None,
                        insert_text: Some(format!("\"{}\"", default)),
                        insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                        insert_text_mode: Some(InsertTextMode::AS_IS),
                        text_edit: None,
                        additional_text_edits: None,
                        command: None,
                        commit_characters: None,
                        data: None,
                        tags: if node.is_obsolete(trainz_build) {
                            Some(vec![CompletionItemTag::DEPRECATED])
                        } else {
                            None
                        },
                    })
                }

                completions
            }
            RuleNodeValue::Float(float) => {
                let mut completions = vec![CompletionItem {
                    label: String::from("Float"),
                    label_details: Some(CompletionItemLabelDetails {
                        detail: None,
                        description: node.description(),
                    }),
                    kind: Some(CompletionItemKind::VALUE),
                    detail: node.details(),
                    documentation: None,
                    deprecated: Some(node.is_obsolete(trainz_build)),
                    preselect: None,
                    sort_text: None,
                    filter_text: None,
                    insert_text: Some(String::from("$1")),
                    insert_text_format: Some(InsertTextFormat::SNIPPET),
                    insert_text_mode: Some(InsertTextMode::AS_IS),
                    text_edit: None,
                    additional_text_edits: None,
                    command: None,
                    commit_characters: None,
                    data: None,
                    tags: if node.is_obsolete(trainz_build) {
                        Some(vec![CompletionItemTag::DEPRECATED])
                    } else {
                        None
                    },
                }];

                if let Some(default) = float.default() {
                    completions.push(CompletionItem {
                        label: format!("{:.01}", default),
                        label_details: Some(CompletionItemLabelDetails {
                            detail: None,
                            description: node.description(),
                        }),
                        kind: Some(CompletionItemKind::VALUE),
                        detail: node.details(),
                        documentation: None,
                        deprecated: Some(node.is_obsolete(trainz_build)),
                        preselect: None,
                        sort_text: None,
                        filter_text: None,
                        insert_text: Some(format!("\"{:.01}\"", default)),
                        insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                        insert_text_mode: Some(InsertTextMode::AS_IS),
                        text_edit: None,
                        additional_text_edits: None,
                        command: None,
                        commit_characters: None,
                        data: None,
                        tags: if node.is_obsolete(trainz_build) {
                            Some(vec![CompletionItemTag::DEPRECATED])
                        } else {
                            None
                        },
                    })
                }

                completions
            }
            RuleNodeValue::Integer(integer) => {
                let mut completions = vec![CompletionItem {
                    label: String::from("Integer"),
                    label_details: Some(CompletionItemLabelDetails {
                        detail: None,
                        description: node.description(),
                    }),
                    kind: Some(CompletionItemKind::VALUE),
                    detail: node.details(),
                    documentation: None,
                    deprecated: Some(node.is_obsolete(trainz_build)),
                    preselect: None,
                    sort_text: None,
                    filter_text: None,
                    insert_text: Some(String::from("$1")),
                    insert_text_format: Some(InsertTextFormat::SNIPPET),
                    insert_text_mode: Some(InsertTextMode::AS_IS),
                    text_edit: None,
                    additional_text_edits: None,
                    command: None,
                    commit_characters: None,
                    data: None,
                    tags: if node.is_obsolete(trainz_build) {
                        Some(vec![CompletionItemTag::DEPRECATED])
                    } else {
                        None
                    },
                }];

                if let Some(default) = integer.default() {
                    completions.push(CompletionItem {
                        label: format!("{:}", default),
                        label_details: Some(CompletionItemLabelDetails {
                            detail: None,
                            description: node.description(),
                        }),
                        kind: Some(CompletionItemKind::VALUE),
                        detail: node.details(),
                        documentation: None,
                        deprecated: Some(node.is_obsolete(trainz_build)),
                        preselect: None,
                        sort_text: None,
                        filter_text: None,
                        insert_text: Some(format!("{}", default)),
                        insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                        insert_text_mode: Some(InsertTextMode::AS_IS),
                        text_edit: None,
                        additional_text_edits: None,
                        command: None,
                        commit_characters: None,
                        data: None,
                        tags: if node.is_obsolete(trainz_build) {
                            Some(vec![CompletionItemTag::DEPRECATED])
                        } else {
                            None
                        },
                    })
                }

                completions
            }
            RuleNodeValue::Bool(boolean) => {
                let mut completions = vec![
                    CompletionItem {
                        label: String::from("True"),
                        label_details: Some(CompletionItemLabelDetails {
                            detail: None,
                            description: node.description(),
                        }),
                        kind: Some(CompletionItemKind::VALUE),
                        detail: node.details(),
                        documentation: None,
                        deprecated: Some(node.is_obsolete(trainz_build)),
                        preselect: None,
                        sort_text: Some(String::from("1true")),
                        filter_text: Some(String::from("1true")),
                        insert_text: Some(String::from("1")),
                        insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                        insert_text_mode: Some(InsertTextMode::AS_IS),
                        text_edit: None,
                        additional_text_edits: None,
                        command: None,
                        commit_characters: None,
                        data: None,
                        tags: if node.is_obsolete(trainz_build) {
                            Some(vec![CompletionItemTag::DEPRECATED])
                        } else {
                            None
                        },
                    },
                    CompletionItem {
                        label: String::from("False"),
                        label_details: Some(CompletionItemLabelDetails {
                            detail: None,
                            description: node.description(),
                        }),
                        kind: Some(CompletionItemKind::VALUE),
                        detail: node.details(),
                        documentation: None,
                        deprecated: Some(node.is_obsolete(trainz_build)),
                        preselect: None,
                        sort_text: Some(String::from("0false")),
                        filter_text: Some(String::from("0false")),
                        insert_text: Some(String::from("0")),
                        insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                        insert_text_mode: Some(InsertTextMode::AS_IS),
                        text_edit: None,
                        additional_text_edits: None,
                        command: None,
                        commit_characters: None,
                        data: None,
                        tags: if node.is_obsolete(trainz_build) {
                            Some(vec![CompletionItemTag::DEPRECATED])
                        } else {
                            None
                        },
                    },
                ];

                if let Some(default) = boolean.default() {
                    completions.push(CompletionItem {
                        label: if *default {
                            String::from("True (default)")
                        } else {
                            String::from("False (default)")
                        },
                        label_details: Some(CompletionItemLabelDetails {
                            detail: None,
                            description: node.description(),
                        }),
                        kind: Some(CompletionItemKind::VALUE),
                        detail: node.details(),
                        documentation: None,
                        deprecated: Some(node.is_obsolete(trainz_build)),
                        preselect: None,
                        sort_text: Some(if *default {
                            String::from("1true")
                        } else {
                            String::from("0false")
                        }),
                        filter_text: Some(if *default {
                            String::from("1true")
                        } else {
                            String::from("0false")
                        }),
                        insert_text: Some(if *default {
                            String::from("1")
                        } else {
                            String::from("0")
                        }),
                        insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                        insert_text_mode: Some(InsertTextMode::AS_IS),
                        text_edit: None,
                        additional_text_edits: None,
                        command: None,
                        commit_characters: None,
                        data: None,
                        tags: if node.is_obsolete(trainz_build) {
                            Some(vec![CompletionItemTag::DEPRECATED])
                        } else {
                            None
                        },
                    })
                }

                completions
            }
            RuleNodeValue::Rgb(rgb) => {
                let mut completions = vec![CompletionItem {
                    label: String::from("0,0,0"),
                    label_details: Some(CompletionItemLabelDetails {
                        detail: None,
                        description: node.description(),
                    }),
                    kind: Some(CompletionItemKind::COLOR),
                    detail: node.details(),
                    documentation: None,
                    deprecated: Some(node.is_obsolete(trainz_build)),
                    preselect: None,
                    sort_text: Some(String::from("0,0,0")),
                    filter_text: Some(String::from("0,0,0")),
                    insert_text: Some(String::from("$1,$2,$3")),
                    insert_text_format: Some(InsertTextFormat::SNIPPET),
                    insert_text_mode: Some(InsertTextMode::AS_IS),
                    text_edit: None,
                    additional_text_edits: None,
                    command: None,
                    commit_characters: None,
                    data: None,
                    tags: if node.is_obsolete(trainz_build) {
                        Some(vec![CompletionItemTag::DEPRECATED])
                    } else {
                        None
                    },
                }];

                if let Some((r, g, b)) = rgb.default() {
                    completions.push(CompletionItem {
                        label: format!("{:0},{:0},{:0}", r, g, b),
                        label_details: Some(CompletionItemLabelDetails {
                            detail: None,
                            description: node.description(),
                        }),
                        kind: Some(CompletionItemKind::COLOR),
                        detail: node.details(),
                        documentation: None,
                        deprecated: Some(node.is_obsolete(trainz_build)),
                        preselect: None,
                        sort_text: Some(format!("{:0},{:0},{:0}", r, g, b)),
                        filter_text: Some(format!("{:0},{:0},{:0}", r, g, b)),
                        insert_text: Some(format!("{:0},{:0},{:0}", r, g, b)),
                        insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                        insert_text_mode: Some(InsertTextMode::AS_IS),
                        text_edit: None,
                        additional_text_edits: None,
                        command: None,
                        commit_characters: None,
                        data: None,
                        tags: if node.is_obsolete(trainz_build) {
                            Some(vec![CompletionItemTag::DEPRECATED])
                        } else {
                            None
                        },
                    })
                }

                completions
            }
            RuleNodeValue::ComboBox(combo) => options_to_completion_item(
                key_value_pair,
                node,
                combo.options(),
                trainz_build,
                true,
            ),
            RuleNodeValue::FloatComboBox(combo) => options_to_completion_item(
                key_value_pair,
                node,
                combo.options(),
                trainz_build,
                false,
            ),
            RuleNodeValue::IntComboBox(combo) => options_to_completion_item(
                key_value_pair,
                node,
                &HashMap::from_par_iter(
                    combo
                        .options()
                        .par_iter()
                        .map(|(key, value)| (format!("{}", key), value.clone())),
                ),
                trainz_build,
                false,
            ),
            RuleNodeValue::ListBox(listbox) => options_to_completion_item(
                key_value_pair,
                node,
                listbox.options(),
                trainz_build,
                true,
            ),
            RuleNodeValue::Kuid(_) => {
                vec![
                    CompletionItem {
                        label: String::from("Kuid"),
                        label_details: Some(CompletionItemLabelDetails {
                            detail: None,
                            description: node.description(),
                        }),
                        kind: Some(CompletionItemKind::VARIABLE),
                        detail: node.details(),
                        documentation: None,
                        deprecated: Some(node.is_obsolete(trainz_build)),
                        preselect: None,
                        sort_text: Some(String::from("<kuid:0:0>")),
                        filter_text: Some(String::from("<kuid:0:0>")),
                        insert_text: Some(String::from("<kuid:$1:$2>")),
                        insert_text_format: Some(InsertTextFormat::SNIPPET),
                        insert_text_mode: Some(InsertTextMode::AS_IS),
                        text_edit: None,
                        additional_text_edits: None,
                        command: None,
                        commit_characters: None,
                        data: None,
                        tags: if node.is_obsolete(trainz_build) {
                            Some(vec![CompletionItemTag::DEPRECATED])
                        } else {
                            None
                        },
                    },
                    CompletionItem {
                        label: String::from("Kuid2"),
                        label_details: Some(CompletionItemLabelDetails {
                            detail: None,
                            description: node.description(),
                        }),
                        kind: Some(CompletionItemKind::VARIABLE),
                        detail: node.details(),
                        documentation: None,
                        deprecated: Some(node.is_obsolete(trainz_build)),
                        preselect: None,
                        sort_text: Some(String::from("<kuid2:0:0:0>")),
                        filter_text: Some(String::from("<kuid2:0:0:0>")),
                        insert_text: Some(String::from("<kuid2:$1:$2:$3>")),
                        insert_text_format: Some(InsertTextFormat::SNIPPET),
                        insert_text_mode: Some(InsertTextMode::AS_IS),
                        text_edit: None,
                        additional_text_edits: None,
                        command: None,
                        commit_characters: None,
                        data: None,
                        tags: if node.is_obsolete(trainz_build) {
                            Some(vec![CompletionItemTag::DEPRECATED])
                        } else {
                            None
                        },
                    },
                ]
            }
            RuleNodeValue::KuidBrowser(kuid) => {
                let mut completions = vec![
                    CompletionItem {
                        label: String::from("Kuid"),
                        label_details: Some(CompletionItemLabelDetails {
                            detail: None,
                            description: node.description(),
                        }),
                        kind: Some(CompletionItemKind::VARIABLE),
                        detail: node.details(),
                        documentation: None,
                        deprecated: Some(node.is_obsolete(trainz_build)),
                        preselect: None,
                        sort_text: Some(String::from("<kuid:0:0>")),
                        filter_text: Some(String::from("<kuid:0:0>")),
                        insert_text: Some(String::from("<kuid:$1:$2>")),
                        insert_text_format: Some(InsertTextFormat::SNIPPET),
                        insert_text_mode: Some(InsertTextMode::AS_IS),
                        text_edit: None,
                        additional_text_edits: None,
                        command: None,
                        commit_characters: None,
                        data: None,
                        tags: if node.is_obsolete(trainz_build) {
                            Some(vec![CompletionItemTag::DEPRECATED])
                        } else {
                            None
                        },
                    },
                    CompletionItem {
                        label: String::from("Kuid2"),
                        label_details: Some(CompletionItemLabelDetails {
                            detail: None,
                            description: node.description(),
                        }),
                        kind: Some(CompletionItemKind::VARIABLE),
                        detail: node.details(),
                        documentation: None,
                        deprecated: Some(node.is_obsolete(trainz_build)),
                        preselect: None,
                        sort_text: Some(String::from("<kuid2:0:0:0>")),
                        filter_text: Some(String::from("<kuid2:0:0:0>")),
                        insert_text: Some(String::from("<kuid2:$1:$2:$3>")),
                        insert_text_format: Some(InsertTextFormat::SNIPPET),
                        insert_text_mode: Some(InsertTextMode::AS_IS),
                        text_edit: None,
                        additional_text_edits: None,
                        command: None,
                        commit_characters: None,
                        data: None,
                        tags: if node.is_obsolete(trainz_build) {
                            Some(vec![CompletionItemTag::DEPRECATED])
                        } else {
                            None
                        },
                    },
                ];

                if let Some((user, content, version)) = kuid.default() {
                    if let Some(version) = version {
                        completions.push(CompletionItem {
                            label: format!("<kuid2:{}:{}:{}>", user, content, version),
                            label_details: Some(CompletionItemLabelDetails {
                                detail: None,
                                description: node.description(),
                            }),
                            kind: Some(CompletionItemKind::REFERENCE),
                            detail: node.details(),
                            documentation: None,
                            deprecated: Some(node.is_obsolete(trainz_build)),
                            preselect: None,
                            sort_text: Some(format!("<kuid2:{}:{}:{}>", user, content, version)),
                            filter_text: Some(format!("<kuid2:{}:{}:{}>", user, content, version)),
                            insert_text: Some(format!("<kuid2:{}:{}:{}>", user, content, version)),
                            insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                            insert_text_mode: Some(InsertTextMode::AS_IS),
                            text_edit: None,
                            additional_text_edits: None,
                            command: None,
                            commit_characters: None,
                            data: None,
                            tags: if node.is_obsolete(trainz_build) {
                                Some(vec![CompletionItemTag::DEPRECATED])
                            } else {
                                None
                            },
                        })
                    } else {
                        completions.push(CompletionItem {
                            label: format!("<kuid:{}:{}>", user, content),
                            label_details: Some(CompletionItemLabelDetails {
                                detail: None,
                                description: node.description(),
                            }),
                            kind: Some(CompletionItemKind::REFERENCE),
                            detail: node.details(),
                            documentation: None,
                            deprecated: Some(node.is_obsolete(trainz_build)),
                            preselect: None,
                            sort_text: Some(format!("<kuid:{}:{}>", user, content)),
                            filter_text: Some(format!("<kuid:{}:{}>", user, content)),
                            insert_text: Some(format!("<kuid:{}:{}>", user, content)),
                            insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                            insert_text_mode: Some(InsertTextMode::AS_IS),
                            text_edit: None,
                            additional_text_edits: None,
                            command: None,
                            commit_characters: None,
                            data: None,
                            tags: if node.is_obsolete(trainz_build) {
                                Some(vec![CompletionItemTag::DEPRECATED])
                            } else {
                                None
                            },
                        })
                    }
                }

                completions
            }
            RuleNodeValue::FilePath(filepath) => {
                let mut completions = vec![CompletionItem {
                    label: String::from("Filepath"),
                    label_details: Some(CompletionItemLabelDetails {
                        detail: None,
                        description: node.description(),
                    }),
                    kind: Some(CompletionItemKind::FILE),
                    detail: node.details(),
                    documentation: None,
                    deprecated: Some(node.is_obsolete(trainz_build)),
                    preselect: None,
                    sort_text: None,
                    filter_text: None,
                    insert_text: Some(String::from("\"$1\"")),
                    insert_text_format: Some(InsertTextFormat::SNIPPET),
                    insert_text_mode: Some(InsertTextMode::AS_IS),
                    text_edit: None,
                    additional_text_edits: None,
                    command: None,
                    commit_characters: None,
                    data: None,
                    tags: if node.is_obsolete(trainz_build) {
                        Some(vec![CompletionItemTag::DEPRECATED])
                    } else {
                        None
                    },
                }];

                if let Some(default) = filepath.default() {
                    completions.push(CompletionItem {
                        label: default.clone(),
                        label_details: Some(CompletionItemLabelDetails {
                            detail: None,
                            description: node.description(),
                        }),
                        kind: Some(CompletionItemKind::FILE),
                        detail: node.details(),
                        documentation: None,
                        deprecated: Some(node.is_obsolete(trainz_build)),
                        preselect: None,
                        sort_text: None,
                        filter_text: None,
                        insert_text: Some(format!("\"{}\"", default)),
                        insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                        insert_text_mode: Some(InsertTextMode::AS_IS),
                        text_edit: None,
                        additional_text_edits: None,
                        command: None,
                        commit_characters: None,
                        data: None,
                        tags: if node.is_obsolete(trainz_build) {
                            Some(vec![CompletionItemTag::DEPRECATED])
                        } else {
                            None
                        },
                    })
                }

                completions
            }
            RuleNodeValue::FloatList(list) => {
                let mut completions = vec![CompletionItem {
                    label: String::from("0,0"),
                    label_details: Some(CompletionItemLabelDetails {
                        detail: None,
                        description: node.description(),
                    }),
                    kind: Some(CompletionItemKind::VALUE),
                    detail: node.details(),
                    documentation: None,
                    deprecated: Some(node.is_obsolete(trainz_build)),
                    preselect: None,
                    sort_text: Some(String::from("0,0")),
                    filter_text: Some(String::from("0,0")),
                    insert_text: Some(String::from("$1,$2")),
                    insert_text_format: Some(InsertTextFormat::SNIPPET),
                    insert_text_mode: Some(InsertTextMode::AS_IS),
                    text_edit: None,
                    additional_text_edits: None,
                    command: None,
                    commit_characters: None,
                    data: None,
                    tags: if node.is_obsolete(trainz_build) {
                        Some(vec![CompletionItemTag::DEPRECATED])
                    } else {
                        None
                    },
                }];

                if let Some(list) = list.default() {
                    completions.push(CompletionItem {
                        label: list
                            .iter()
                            .map(|item| format!("{:.01}", item))
                            .collect::<Vec<String>>()
                            .join(","),
                        label_details: Some(CompletionItemLabelDetails {
                            detail: None,
                            description: node.description(),
                        }),
                        kind: Some(CompletionItemKind::VALUE),
                        detail: node.details(),
                        documentation: None,
                        deprecated: Some(node.is_obsolete(trainz_build)),
                        preselect: None,
                        sort_text: Some(
                            list.iter()
                                .map(|item| format!("{:.01}", item))
                                .collect::<Vec<String>>()
                                .join(","),
                        ),
                        filter_text: Some(
                            list.iter()
                                .map(|item| format!("{:.01}", item))
                                .collect::<Vec<String>>()
                                .join(","),
                        ),
                        insert_text: Some(
                            list.iter()
                                .map(|item| format!("{:.01}", item))
                                .collect::<Vec<String>>()
                                .join(","),
                        ),
                        insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                        insert_text_mode: Some(InsertTextMode::AS_IS),
                        text_edit: None,
                        additional_text_edits: None,
                        command: None,
                        commit_characters: None,
                        data: None,
                        tags: if node.is_obsolete(trainz_build) {
                            Some(vec![CompletionItemTag::DEPRECATED])
                        } else {
                            None
                        },
                    })
                }

                completions
            }
            RuleNodeValue::Vector2(list) => {
                let mut completions = vec![CompletionItem {
                    label: String::from("0,0"),
                    label_details: Some(CompletionItemLabelDetails {
                        detail: None,
                        description: node.description(),
                    }),
                    kind: Some(CompletionItemKind::VALUE),
                    detail: node.details(),
                    documentation: None,
                    deprecated: Some(node.is_obsolete(trainz_build)),
                    preselect: None,
                    sort_text: Some(String::from("0,0")),
                    filter_text: Some(String::from("0,0")),
                    insert_text: Some(String::from("$1,$2")),
                    insert_text_format: Some(InsertTextFormat::SNIPPET),
                    insert_text_mode: Some(InsertTextMode::AS_IS),
                    text_edit: None,
                    additional_text_edits: None,
                    command: None,
                    commit_characters: None,
                    data: None,
                    tags: if node.is_obsolete(trainz_build) {
                        Some(vec![CompletionItemTag::DEPRECATED])
                    } else {
                        None
                    },
                }];

                if let Some((a, b)) = list.default() {
                    completions.push(CompletionItem {
                        label: format!("{:.01},{:.01}", a, b),
                        label_details: Some(CompletionItemLabelDetails {
                            detail: None,
                            description: node.description(),
                        }),
                        kind: Some(CompletionItemKind::VALUE),
                        detail: node.details(),
                        documentation: None,
                        deprecated: Some(node.is_obsolete(trainz_build)),
                        preselect: None,
                        sort_text: Some(format!("{:.01},{:.01}", a, b)),
                        filter_text: Some(format!("{:.01},{:.01}", a, b)),
                        insert_text: Some(format!("{:.01},{:.01}", a, b)),
                        insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                        insert_text_mode: Some(InsertTextMode::AS_IS),
                        text_edit: None,
                        additional_text_edits: None,
                        command: None,
                        commit_characters: None,
                        data: None,
                        tags: if node.is_obsolete(trainz_build) {
                            Some(vec![CompletionItemTag::DEPRECATED])
                        } else {
                            None
                        },
                    })
                }

                completions
            }
            RuleNodeValue::Vector3(list) => {
                let mut completions = vec![CompletionItem {
                    label: String::from("0,0,0"),
                    label_details: Some(CompletionItemLabelDetails {
                        detail: None,
                        description: node.description(),
                    }),
                    kind: Some(CompletionItemKind::VALUE),
                    detail: node.details(),
                    documentation: None,
                    deprecated: Some(node.is_obsolete(trainz_build)),
                    preselect: None,
                    sort_text: Some(String::from("0,0,0")),
                    filter_text: Some(String::from("0,0,0")),
                    insert_text: Some(String::from("$1,$2,$3")),
                    insert_text_format: Some(InsertTextFormat::SNIPPET),
                    insert_text_mode: Some(InsertTextMode::AS_IS),
                    text_edit: None,
                    additional_text_edits: None,
                    command: None,
                    commit_characters: None,
                    data: None,
                    tags: if node.is_obsolete(trainz_build) {
                        Some(vec![CompletionItemTag::DEPRECATED])
                    } else {
                        None
                    },
                }];

                if let Some((a, b, c)) = list.default() {
                    completions.push(CompletionItem {
                        label: format!("{:.01},{:.01},{:.01}", a, b, c),
                        label_details: Some(CompletionItemLabelDetails {
                            detail: None,
                            description: node.description(),
                        }),
                        kind: Some(CompletionItemKind::VALUE),
                        detail: node.details(),
                        documentation: None,
                        deprecated: Some(node.is_obsolete(trainz_build)),
                        preselect: None,
                        sort_text: Some(format!("{:.01},{:.01},{:.01}", a, b, c)),
                        filter_text: Some(format!("{:.01},{:.01},{:.01}", a, b, c)),
                        insert_text: Some(format!("{:.01},{:.01},{:.01}", a, b, c)),
                        insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                        insert_text_mode: Some(InsertTextMode::AS_IS),
                        text_edit: None,
                        additional_text_edits: None,
                        command: None,
                        commit_characters: None,
                        data: None,
                        tags: if node.is_obsolete(trainz_build) {
                            Some(vec![CompletionItemTag::DEPRECATED])
                        } else {
                            None
                        },
                    })
                }

                completions
            }
            RuleNodeValue::Vector4(list) => {
                let mut completions = vec![CompletionItem {
                    label: String::from("0,0,0,0"),
                    label_details: Some(CompletionItemLabelDetails {
                        detail: None,
                        description: node.description(),
                    }),
                    kind: Some(CompletionItemKind::VALUE),
                    detail: node.details(),
                    documentation: None,
                    deprecated: Some(node.is_obsolete(trainz_build)),
                    preselect: None,
                    sort_text: Some(String::from("0,0,0,0")),
                    filter_text: Some(String::from("0,0,0,0")),
                    insert_text: Some(String::from("$1,$2,$3,$4")),
                    insert_text_format: Some(InsertTextFormat::SNIPPET),
                    insert_text_mode: Some(InsertTextMode::AS_IS),
                    text_edit: None,
                    additional_text_edits: None,
                    command: None,
                    commit_characters: None,
                    data: None,
                    tags: if node.is_obsolete(trainz_build) {
                        Some(vec![CompletionItemTag::DEPRECATED])
                    } else {
                        None
                    },
                }];

                if let Some((a, b, c, d)) = list.default() {
                    completions.push(CompletionItem {
                        label: format!("{:.01},{:.01},{:.01},{:.01}", a, b, c, d),
                        label_details: Some(CompletionItemLabelDetails {
                            detail: None,
                            description: node.description(),
                        }),
                        kind: Some(CompletionItemKind::VALUE),
                        detail: node.details(),
                        documentation: None,
                        deprecated: Some(node.is_obsolete(trainz_build)),
                        preselect: None,
                        sort_text: Some(format!("{:.01},{:.01},{:.01},{:.01}", a, b, c, d)),
                        filter_text: Some(format!("{:.01},{:.01},{:.01},{:.01}", a, b, c, d)),
                        insert_text: Some(format!("{:.01},{:.01},{:.01},{:.01}", a, b, c, d)),
                        insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                        insert_text_mode: Some(InsertTextMode::AS_IS),
                        text_edit: None,
                        additional_text_edits: None,
                        command: None,
                        commit_characters: None,
                        data: None,
                        tags: if node.is_obsolete(trainz_build) {
                            Some(vec![CompletionItemTag::DEPRECATED])
                        } else {
                            None
                        },
                    })
                }

                completions
            }
            RuleNodeValue::Vector5(list) => {
                let mut completions = vec![CompletionItem {
                    label: String::from("0,0,0,0,0"),
                    label_details: Some(CompletionItemLabelDetails {
                        detail: None,
                        description: node.description(),
                    }),
                    kind: Some(CompletionItemKind::VALUE),
                    detail: node.details(),
                    documentation: None,
                    deprecated: Some(node.is_obsolete(trainz_build)),
                    preselect: None,
                    sort_text: Some(String::from("0,0,0,0,0")),
                    filter_text: Some(String::from("0,0,0,0,0")),
                    insert_text: Some(String::from("$1,$2,$3,$4,$5")),
                    insert_text_format: Some(InsertTextFormat::SNIPPET),
                    insert_text_mode: Some(InsertTextMode::AS_IS),
                    text_edit: None,
                    additional_text_edits: None,
                    command: None,
                    commit_characters: None,
                    data: None,
                    tags: if node.is_obsolete(trainz_build) {
                        Some(vec![CompletionItemTag::DEPRECATED])
                    } else {
                        None
                    },
                }];

                if let Some((a, b, c, d, e)) = list.default() {
                    completions.push(CompletionItem {
                        label: format!("{:.01},{:.01},{:.01},{:.01},{:.01}", a, b, c, d, e),
                        label_details: Some(CompletionItemLabelDetails {
                            detail: None,
                            description: node.description(),
                        }),
                        kind: Some(CompletionItemKind::VALUE),
                        detail: node.details(),
                        documentation: None,
                        deprecated: Some(node.is_obsolete(trainz_build)),
                        preselect: None,
                        sort_text: Some(format!(
                            "{:.01},{:.01},{:.01},{:.01},{:.01}",
                            a, b, c, d, e
                        )),
                        filter_text: Some(format!(
                            "{:.01},{:.01},{:.01},{:.01},{:.01}",
                            a, b, c, d, e
                        )),
                        insert_text: Some(format!(
                            "{:.01},{:.01},{:.01},{:.01},{:.01}",
                            a, b, c, d, e
                        )),
                        insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                        insert_text_mode: Some(InsertTextMode::AS_IS),
                        text_edit: None,
                        additional_text_edits: None,
                        command: None,
                        commit_characters: None,
                        data: None,
                        tags: if node.is_obsolete(trainz_build) {
                            Some(vec![CompletionItemTag::DEPRECATED])
                        } else {
                            None
                        },
                    })
                }

                completions
            }
            RuleNodeValue::Vector6(list) => {
                let mut completions = vec![CompletionItem {
                    label: String::from("0,0,0,0,0,0"),
                    label_details: Some(CompletionItemLabelDetails {
                        detail: None,
                        description: node.description(),
                    }),
                    kind: Some(CompletionItemKind::VALUE),
                    detail: node.details(),
                    documentation: None,
                    deprecated: Some(node.is_obsolete(trainz_build)),
                    preselect: None,
                    sort_text: Some(String::from("0,0,0,0,0,0")),
                    filter_text: Some(String::from("0,0,0,0,0,0")),
                    insert_text: Some(String::from("$1,$2,$3,$4,$5,$6")),
                    insert_text_format: Some(InsertTextFormat::SNIPPET),
                    insert_text_mode: Some(InsertTextMode::AS_IS),
                    text_edit: None,
                    additional_text_edits: None,
                    command: None,
                    commit_characters: None,
                    data: None,
                    tags: if node.is_obsolete(trainz_build) {
                        Some(vec![CompletionItemTag::DEPRECATED])
                    } else {
                        None
                    },
                }];

                if let Some((a, b, c, d, e, f)) = list.default() {
                    completions.push(CompletionItem {
                        label: format!(
                            "{:.01},{:.01},{:.01},{:.01},{:.01},{:.01}",
                            a, b, c, d, e, f
                        ),
                        label_details: Some(CompletionItemLabelDetails {
                            detail: None,
                            description: node.description(),
                        }),
                        kind: Some(CompletionItemKind::VALUE),
                        detail: node.details(),
                        documentation: None,
                        deprecated: Some(node.is_obsolete(trainz_build)),
                        preselect: None,
                        sort_text: Some(format!(
                            "{:.01},{:.01},{:.01},{:.01},{:.01},{:.01}",
                            a, b, c, d, e, f
                        )),
                        filter_text: Some(format!(
                            "{:.01},{:.01},{:.01},{:.01},{:.01},{:.01}",
                            a, b, c, d, e, f
                        )),
                        insert_text: Some(format!(
                            "{:.01},{:.01},{:.01},{:.01},{:.01},{:.01}",
                            a, b, c, d, e, f
                        )),
                        insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                        insert_text_mode: Some(InsertTextMode::AS_IS),
                        text_edit: None,
                        additional_text_edits: None,
                        command: None,
                        commit_characters: None,
                        data: None,
                        tags: if node.is_obsolete(trainz_build) {
                            Some(vec![CompletionItemTag::DEPRECATED])
                        } else {
                            None
                        },
                    })
                }

                completions
            }
        }
    }

    if key_value_pair.is_some_and(|kvp| kvp.value.is_some()) {
        build_completion_items_for_value(key_value_pair, node, value, trainz_build)
    } else {
        build_completion_item(node, trainz_build)
    }
}

#[tracing::instrument(skip(tag_array, label, trainz_build))]
fn build_tag_array_completion_item(
    tag_array: &Arc<RuleNode>,
    label: Option<String>,
    trainz_build: &f64,
) -> Vec<CompletionItem> {
    debug!("Building tag completion items {:?}", tag_array);

    vec![CompletionItem {
        label: label.clone().unwrap_or(tag_array.name()),
        label_details: Some(CompletionItemLabelDetails {
            detail: None,
            description: tag_array.description(),
        }),
        kind: Some(CompletionItemKind::KEYWORD),
        detail: tag_array.details(),
        documentation: tag_array.documentation(trainz_build).map(|value| {
            let result = value.join("\n\n");
            Documentation::MarkupContent(MarkupContent {
                kind: MarkupKind::Markdown,
                value: result,
            })
        }),
        deprecated: Some(tag_array.is_obsolete(trainz_build)),
        preselect: None,
        sort_text: label.clone(),
        filter_text: label,
        insert_text: None,
        insert_text_format: None,
        insert_text_mode: None,
        text_edit: None,
        additional_text_edits: None,
        command: None,
        commit_characters: None,
        data: None,
        tags: if tag_array.is_obsolete(trainz_build) {
            Some(vec![CompletionItemTag::DEPRECATED])
        } else {
            None
        },
    }]
}
