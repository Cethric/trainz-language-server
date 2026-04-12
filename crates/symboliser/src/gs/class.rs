use tower_lsp_server::ls_types::{DocumentSymbol, SymbolKind, SymbolTag};
use trainz_ast::gs::{ClassDef, MethodDef, NativeMethodDef};
use trainz_common::range::clamp_range;

use super::stmt::process_block;
use crate::gs::util::{is_class_obsolete, is_method_obsolete};

#[allow(deprecated)]
pub(crate) fn process_class_symbol(class: &ClassDef) -> DocumentSymbol {
    let mut children = vec![];

    for field in &class.fields {
        for name in &field.names {
            children.push(DocumentSymbol {
                name: name.name.clone(),
                detail: Some(format!("{:?}", field.ty)),
                kind: SymbolKind::PROPERTY,
                tags: None,
                deprecated: None,
                range: field.range,
                selection_range: clamp_range(&field.range, name.range),
                children: None,
            });
        }
    }

    for method in &class.methods {
        children.push(process_method_symbol(method));
    }

    for method in &class.native_methods {
        children.push(process_native_method_symbol(method));
    }

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
fn process_method_symbol(method: &MethodDef) -> DocumentSymbol {
    let deprecated = is_method_obsolete(&method.modifiers);

    let mut children = vec![];
    for param in &method.params {
        children.push(DocumentSymbol {
            name: param.name.name.clone(),
            detail: Some(format!("{:?}", param.ty)),
            kind: SymbolKind::VARIABLE,
            tags: None,
            deprecated: None,
            range: param.range,
            selection_range: clamp_range(&param.range, param.name.range),
            children: None,
        });
    }
    children.extend(process_block(&method.body));

    DocumentSymbol {
        name: method.name.name.clone(),
        detail: Some(format!("{:?}", method.return_type)),
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

#[allow(deprecated)]
fn process_native_method_symbol(method: &NativeMethodDef) -> DocumentSymbol {
    let deprecated = is_method_obsolete(&method.modifiers);

    let mut children = vec![];
    for param in &method.params {
        children.push(DocumentSymbol {
            name: param.name.name.clone(),
            detail: Some(format!("{:?}", param.ty)),
            kind: SymbolKind::VARIABLE,
            tags: None,
            deprecated: None,
            range: param.range,
            selection_range: clamp_range(&param.range, param.name.range),
            children: None,
        });
    }

    DocumentSymbol {
        name: method.name.name.clone(),
        detail: Some(format!("{:?}", method.return_type)),
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
