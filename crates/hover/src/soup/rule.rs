use rayon::prelude::*;
use tower_lsp_server::ls_types::{Hover, MarkupContent, MarkupKind};
use trainz_common::wiki::get_wiki_kind_name;
use trainz_soup_validators::ContainerRule;

#[tracing::instrument]
pub fn create_hover_from_rule(rule: &ContainerRule, range: &trainz_ast::Range) -> Option<Hover> {
    let mut doc = format!("### Key: `{}`\n", rule.key);
    if let Some(t) = &rule.type_name {
        doc.push_str(&format!("**Type**: `{}`\n\n", t));
    }

    if rule.key.eq_ignore_ascii_case("kind") {
        if let Some(kind_name) = &rule.kind {
            let wiki_name = get_wiki_kind_name(kind_name);
            doc.push_str(&format!(
                "**Wiki**: [KIND {}](https://online.ts2009.com/mediaWiki/index.php/KIND_{})\n\n",
                wiki_name, wiki_name
            ));
        }
    } else if let Some(k) = &rule.kind {
        doc.push_str(&format!("**Kind**: `{}`\n\n", k));
    }
    if let Some(d) = &rule.description
        && !d.is_empty()
    {
        doc.push_str(&format!("**Description**: {}\n\n", d));
    }

    let mut validation_rules = Vec::new();

    if let Some(v) = &rule.validation {
        validation_rules.push(format!(
            "**Validation**:\n{}",
            v.par_iter()
                .map(|v| format!("- `{}`", v))
                .collect::<Vec<String>>()
                .join("\n")
        ));
    }
    if let Some(d) = &rule.default_value {
        validation_rules.push(format!("**Default**: `{}`", d));
    }
    if let Some(c) = &rule.compulsory {
        validation_rules.push(format!("**Compulsory**: `{}`", c));
    }
    if let Some(f) = &rule.filter {
        validation_rules.push(format!("**Filter**: `{}`", f));
    }
    if let Some(dis) = &rule.disabled
        && *dis
    {
        validation_rules.push("**Disabled**: `true`".to_string());
    }

    if !validation_rules.is_empty() {
        doc.push_str("#### Validation Rules\n");
        for v_rule in validation_rules {
            doc.push_str(&format!("- {}\n", v_rule));
        }
        doc.push('\n');
    }

    Some(Hover {
        contents: tower_lsp_server::ls_types::HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: doc,
        }),
        range: Some(*range),
    })
}
