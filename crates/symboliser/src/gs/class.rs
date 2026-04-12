use tower_lsp_server::ls_types::{DocumentSymbol, SymbolKind, SymbolTag};
use trainz_ast::gs::{ClassDef, MethodDef};
use trainz_common::range::clamp_range;

use super::stmt::process_block;
use crate::gs::util::{is_class_obsolete, is_method_obsolete};

#[allow(deprecated)]
#[tracing::instrument(skip(resolver))]
pub(crate) fn process_class_symbol(
    class: &ClassDef,
    program: &trainz_ast::gs::Program,
    resolver: &dyn trainz_ast::gs::type_eval::ClassResolver,
) -> DocumentSymbol {
    let mut children = vec![];

    for field in class.fields.values() {
        children.extend(super::expr::process_type_symbols(&field.ty));
        children.push(DocumentSymbol {
            name: field.name.name.clone(),
            detail: Some(format!("{}", field.ty)),
            kind: SymbolKind::PROPERTY,
            tags: None,
            deprecated: None,
            range: field.range,
            selection_range: clamp_range(&field.range, field.name.range),
            children: None,
        });
    }

    for methods in class.methods.values() {
        for method in methods {
            children.push(process_method_symbol(method, class, program, resolver));
        }
    }

    children.sort_by_key(|s| (s.range.start, s.selection_range.start));

    let deprecated = is_class_obsolete(&class.modifiers);

    DocumentSymbol {
        name: class.name.name.clone(),
        detail: None,
        kind: SymbolKind::CLASS,
        tags: if deprecated {
            Some(vec![SymbolTag::DEPRECATED])
        } else {
            None
        },
        deprecated: Some(deprecated),
        range: class.range,
        selection_range: clamp_range(&class.range, class.name.range),
        children: if children.is_empty() {
            None
        } else {
            Some(children)
        },
    }
}

#[allow(deprecated)]
#[tracing::instrument(skip(resolver))]
fn process_method_symbol(
    method: &MethodDef,
    class: &ClassDef,
    program: &trainz_ast::gs::Program,
    resolver: &dyn trainz_ast::gs::type_eval::ClassResolver,
) -> DocumentSymbol {
    let deprecated = is_method_obsolete(&method.modifiers);
    let is_native = is_method_native(&method.modifiers);

    let mut children = vec![];
    if let trainz_ast::gs::TypeOrVoid::Type(ty) = &method.return_type {
        children.extend(super::expr::process_type_symbols(ty));
    }
    for param in &method.params {
        children.extend(super::expr::process_type_symbols(&param.ty));
        children.push(DocumentSymbol {
            name: param.name.name.clone(),
            detail: Some(format!("{}", param.ty)),
            kind: SymbolKind::VARIABLE,
            tags: None,
            deprecated: None,
            range: param.range,
            selection_range: clamp_range(&param.range, param.name.range),
            children: None,
        });
    }
    if let Some(body) = &method.body {
        children.extend(process_block(body, program, resolver));
    }

    use crate::gs::util::is_method_native;
    let has_separate_declaration = class
        .methods
        .get(&method.name.name)
        .map(|ms| {
            ms.iter().any(|m| {
                m.body.is_none() && !is_method_native(&m.modifiers) && m.range != method.range
            })
        })
        .unwrap_or(false);

    let detail = if is_native {
        format!("{} (declaration, definition)", method.return_type)
    } else if method.body.is_some() {
        if has_separate_declaration {
            format!("{} (definition)", method.return_type)
        } else {
            format!("{} (declaration, definition)", method.return_type)
        }
    } else {
        format!("{} (declaration)", method.return_type)
    };

    DocumentSymbol {
        name: method.name.name.clone(),
        detail: Some(detail),
        kind: SymbolKind::METHOD,
        tags: if deprecated {
            Some(vec![SymbolTag::DEPRECATED])
        } else {
            None
        },
        deprecated: Some(deprecated),
        range: method.range,
        selection_range: clamp_range(&method.range, method.name.range),
        children: if children.is_empty() {
            None
        } else {
            Some(children)
        },
    }
}
