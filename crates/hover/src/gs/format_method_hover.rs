/// Formats a `MethodDef` into a string suitable for hover information.
///
/// # Arguments
///
/// * `method` - The `MethodDef` to format.
///
/// # Returns
///
/// A `String` containing the formatted method signature.
///
/// # Example
///
/// ```
/// # use trainz_ast::gs::{MethodDef, Type};
/// # use trainz_ast::Identifier;
/// # let method = MethodDef {
/// #     modifiers: vec![],
/// #     parent_class: None,
/// #     name: Identifier { name: "myMethod".to_string(), range: Default::default() },
/// #     return_type: Type::Void,
/// #     params: vec![],
/// #     void_param_range: Some(Default::default()),
/// #     range: Default::default(),
/// # };
/// # let hover = trainz_hover::gs::format_method_hover::format_method_hover(&method);
/// ```
pub fn format_method_hover(method: &trainz_ast::gs::MethodDef) -> String {
    let mut value = String::new();
    for (modifier, _) in &method.modifiers {
        value.push_str(&format!("{} ", modifier));
    }

    let class_prefix = if let Some(parent) = &method.parent_class {
        format!("{}::", parent)
    } else {
        "".to_string()
    };

    value.push_str(&format!(
        "{} {}{}(",
        method.return_type, class_prefix, method.name.name
    ));
    if method.void_param_range.is_some() {
        value.push_str("void");
    } else {
        for (i, param) in method.params.iter().enumerate() {
            if i > 0 {
                value.push_str(", ");
            }
            value.push_str(&format!("{} {}", param.ty, param.name.name));
        }
    }
    value.push(')');
    value
}
