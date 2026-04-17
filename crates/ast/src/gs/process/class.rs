use crate::gs::process::expr::process_expr;
use crate::gs::process::helpers::{
    create_block, process_identifier, process_type, process_type_or_void, push_scope,
};
use crate::gs::process::stmt::process_statements;
use crate::gs::{
    ClassDef, ClassModifier, FieldDef, FieldModifier, Identifier, MethodDef, MethodModifier, Param,
    Scope,
};
use pest::iterators::Pair;
use std::collections::HashMap;
use tower_lsp_server::ls_types::Range;
use tracing::trace;
use trainz_common::range::{combine_ranges, pair_to_range};
use trainz_parser::gs::grammar::Rule;

#[tracing::instrument(skip(scopes, parent_scope_id, class_definition))]
pub fn process_class_definition(
    scopes: &mut Vec<Scope>,
    parent_scope_id: usize,
    class_definition: Pair<Rule>,
) -> Option<ClassDef> {
    let range = pair_to_range(&class_definition);
    let inner = class_definition.into_inner();

    let scope_id = push_scope(scopes, Some(parent_scope_id), range, vec![]);

    let mut class_def = ClassDef {
        modifiers: vec![],
        keyword_class_range: Range::default(),
        name: Identifier {
            name: "".to_string(),
            range: Range::default(),
        },
        keyword_is_class_range: None,
        superclasses: vec![],
        fields: HashMap::new(),
        methods: HashMap::new(),
        body_range: Range::default(),
        scope_id,
        range,
    };

    process_class_inner(scopes, &mut class_def, inner);

    Some(class_def)
}

#[tracing::instrument(skip(scopes, class_def, inner))]
fn process_class_inner(
    scopes: &mut Vec<Scope>,
    class_def: &mut ClassDef,
    inner: pest::iterators::Pairs<Rule>,
) {
    for pair in inner {
        let rule = pair.as_rule();

        match rule {
            Rule::class_modifiers => {
                let modifiers: Vec<(ClassModifier, Range)> = pair
                    .into_inner()
                    .filter_map(|pair| {
                        let r = pair_to_range(&pair);
                        match pair.as_rule() {
                            Rule::keyword_final => Some((ClassModifier::Final, r)),
                            Rule::keyword_game => Some((ClassModifier::Game, r)),
                            Rule::keyword_static => Some((ClassModifier::Static, r)),
                            Rule::keyword_secured => Some((ClassModifier::Secured, r)),
                            Rule::obsolete_statement => {
                                let obsolete_val = pair
                                    .into_inner()
                                    .filter(|p| p.as_rule() == Rule::integer)
                                    .filter_map(|i| i.as_str().parse::<i64>().ok())
                                    .next();
                                Some((ClassModifier::Obsolete(obsolete_val), r))
                            }
                            _ => None,
                        }
                    })
                    .collect();
                trace!("process_class_definition, modifiers: {:?}", modifiers);
                class_def.modifiers = modifiers;
            }
            Rule::class_name => {
                class_def.name = process_identifier(pair);
            }
            Rule::keyword_class => {
                class_def.keyword_class_range = pair_to_range(&pair);
            }
            Rule::class_inheritance => {
                let inner = pair.into_inner();
                // peek first to see if it's keyword_is_class
                let first_pair = inner.clone().next();
                if let Some(first) = first_pair
                    && first.as_rule() == Rule::keyword_is_class
                {
                    class_def.keyword_is_class_range = Some(pair_to_range(&first));
                }

                let inheritance: Vec<Identifier> = inner
                    .filter_map(|pair| match pair.as_rule() {
                        Rule::superclass_name => Some(process_identifier(pair)),
                        _ => None,
                    })
                    .collect();

                trace!("process_class_definition, inheritance: {:?}", inheritance);
                class_def.superclasses = inheritance;
            }
            Rule::class_body => {
                class_def.body_range = pair_to_range(&pair);
                process_class_inner(scopes, class_def, pair.into_inner());
            }
            Rule::class_member => {
                let fields = process_field_definition(pair);
                for mut field in fields {
                    field.parent_class = Some(class_def.name.name.clone());
                    scopes[class_def.scope_id]
                        .variables
                        .push((field.ty.clone(), field.name.clone()));
                    class_def.fields.insert(field.name.name.clone(), field);
                }
            }
            Rule::class_method => {
                let mut method = process_method_definition(scopes, class_def.scope_id, pair);
                method.parent_class = Some(class_def.name.name.clone());
                class_def
                    .methods
                    .entry(method.name.name.clone())
                    .or_default()
                    .push(method);
            }
            _ => {}
        }
    }
}

#[tracing::instrument(skip(pair))]
fn process_field_definition(pair: Pair<Rule>) -> Vec<FieldDef> {
    let range = pair_to_range(&pair);
    let mut inner = pair.into_inner();
    let modifiers_pair = inner.next().unwrap();
    let modifiers: Vec<(FieldModifier, Range)> = modifiers_pair
        .into_inner()
        .filter_map(|p| {
            let r = pair_to_range(&p);
            match p.as_rule() {
                Rule::keyword_static => Some((FieldModifier::Static, r)),
                Rule::keyword_public => Some((FieldModifier::Public, r)),
                Rule::keyword_define => Some((FieldModifier::Define, r)),
                Rule::obsolete_statement => {
                    let val = p
                        .into_inner()
                        .find(|ip| ip.as_rule() == Rule::integer)
                        .map(|ip| ip.as_str().parse().unwrap());
                    Some((FieldModifier::Obsolete(val), r))
                }
                _ => None,
            }
        })
        .collect();

    let ty = process_type(inner.next().unwrap());
    let mut fields = vec![];

    let mut names = vec![];
    let mut initializers = vec![];

    for p in inner {
        match p.as_rule() {
            Rule::class_member_name => names.push(process_identifier(p)),
            Rule::assignment_expr => {
                initializers.push(process_expr(p));
            }
            _ => {}
        }
    }

    for (i, name) in names.into_iter().enumerate() {
        let initializer = initializers.get(i).cloned();
        fields.push(FieldDef {
            parent_class: None,
            modifiers: modifiers.clone(),
            ty: ty.clone(),
            name,
            initializer,
            range,
        });
    }

    fields
}

#[tracing::instrument(skip(scopes, parent_scope_id, pair))]
fn process_method_definition(
    scopes: &mut Vec<Scope>,
    parent_scope_id: usize,
    pair: Pair<Rule>,
) -> MethodDef {
    let range = pair_to_range(&pair);
    let mut inner = pair.into_inner();
    let modifiers = process_method_modifiers(inner.next().unwrap());
    let return_type = process_type_or_void(inner.next().unwrap());
    let name = process_identifier(inner.next().unwrap());
    let (params, void_param_range) = process_params(inner.next().unwrap());

    let scope_id = push_scope(
        scopes,
        Some(parent_scope_id),
        range,
        params
            .iter()
            .map(|p| (p.ty.clone(), p.name.clone()))
            .collect(),
    );

    let body = if let Some(body_pair) = inner.next() {
        let body_range = pair_to_range(&body_pair);
        trace!(
            "process_method_definition: body_pair rule: {:?}",
            body_pair.as_rule()
        );

        let statements = if body_pair.as_rule() == Rule::method_body {
            // method_body is '{' ~ statements ~ '}'
            let body_inner = body_pair.into_inner();
            let mut statements = vec![];
            for pair in body_inner {
                trace!(
                    "process_method_definition: body_inner pair rule: {:?}",
                    pair.as_rule()
                );
                if pair.as_rule() == Rule::statements {
                    statements.extend(process_statements(scopes, scope_id, pair));
                }
            }
            statements
        } else {
            process_statements(scopes, scope_id, body_pair)
        };

        Some(create_block(scope_id, statements, body_range))
    } else {
        None
    };

    MethodDef {
        parent_class: None,
        modifiers,
        return_type,
        name,
        params,
        void_param_range,
        body,
        scope_id,
        range,
    }
}

#[tracing::instrument(skip(pair))]
fn process_method_modifiers(pair: Pair<Rule>) -> Vec<(MethodModifier, Range)> {
    pair.into_inner()
        .filter_map(|p| {
            let r = pair_to_range(&p);
            match p.as_rule() {
                Rule::keyword_static => Some((MethodModifier::Static, r)),
                Rule::keyword_public => Some((MethodModifier::Public, r)),
                Rule::keyword_thread => Some((MethodModifier::Thread, r)),
                Rule::keyword_legacy_compatibility => {
                    Some((MethodModifier::LegacyCompatibility, r))
                }
                Rule::keyword_mandatory => Some((MethodModifier::Mandatory, r)),
                Rule::keyword_native => Some((MethodModifier::Native, r)),
                Rule::obsolete_statement => {
                    let val = p
                        .into_inner()
                        .find(|ip| ip.as_rule() == Rule::integer)
                        .map(|ip| ip.as_str().parse().unwrap());
                    Some((MethodModifier::Obsolete(val), r))
                }
                _ => None,
            }
        })
        .collect()
}

#[tracing::instrument(skip(pair))]
fn process_params(pair: Pair<Rule>) -> (Vec<Param>, Option<Range>) {
    let mut params = vec![];
    let mut void_param_range = None;
    let inner = pair.into_inner();
    let mut flattened_inner = vec![];
    for p in inner {
        flattened_inner.push(p);
    }

    let mut it = flattened_inner.into_iter();
    while let Some(p) = it.next() {
        let r = pair_to_range(&p);
        match p.as_rule() {
            Rule::class_method_parameter_type => {
                let ty = process_type(p);
                // Skip comments between type and name
                let mut name_pair = it.next();
                while let Some(ref np) = name_pair {
                    if np.as_rule() == Rule::line_comment || np.as_rule() == Rule::block_comment {
                        name_pair = it.next();
                    } else {
                        break;
                    }
                }

                if let Some(name_pair) = name_pair {
                    // It might be 'void' which is a type, but the grammar says:
                    // class_method_parameters       =  { paren_open ~ (type_void | class_method_parameter_group)? ~ paren_close }
                    // class_method_parameter_group  = _{ class_method_parameter ~ (comma ~ class_method_parameter)* }
                    // class_method_parameter        = _{ class_method_parameter_type ~ class_method_parameter_name }
                    // If we have 'void', it matches type_void, not class_method_parameter_group.

                    if name_pair.as_rule() == Rule::class_method_parameter_name {
                        let name = process_identifier(name_pair);
                        params.push(Param {
                            ty,
                            name: name.clone(),
                            range: combine_ranges(r, name.range),
                        });
                    }
                }
            }
            Rule::type_void => {
                void_param_range = Some(r);
            }
            Rule::line_comment | Rule::block_comment => {}
            _ => {}
        }
    }
    (params, void_param_range)
}

#[cfg(test)]
mod tests {
    use super::*;
    use trainz_parser::gs::grammar::Rule;

    #[test]
    fn test_obsolete_modifiers() {
        let code = "obsolete\nclass OldClass {\n    obsolete(123)\n    int oldField;\n    obsolete\n    void oldMethod() {}\n};\n";
        let pairs = trainz_parser::gs::parse(code).unwrap();

        let class_pair = pairs
            .into_iter()
            .find(|p| p.as_rule() == Rule::class_definition)
            .unwrap();

        let mut scopes = vec![];
        let root_scope_id = push_scope(&mut scopes, None, Range::default(), vec![]);
        let class_def = process_class_definition(&mut scopes, root_scope_id, class_pair).unwrap();

        assert_eq!(class_def.modifiers.len(), 1);
        match class_def.modifiers[0].0 {
            ClassModifier::Obsolete(None) => {
                let r = class_def.modifiers[0].1;
                assert_eq!(r.start.line, 0);
                assert_eq!(r.end.line, 0);
            }
            _ => panic!("Expected Obsolete(None) for class"),
        }

        assert_eq!(class_def.fields.len(), 1);
        let field = class_def.fields.get("oldField").unwrap();
        assert_eq!(field.modifiers.len(), 1);
        match field.modifiers[0].0 {
            FieldModifier::Obsolete(Some(123)) => {
                let r = field.modifiers[0].1;
                assert_eq!(r.start.line, 2);
                assert_eq!(r.end.line, 2);
            }
            _ => panic!("Expected Obsolete(Some(123)) for field"),
        }

        assert_eq!(class_def.methods.len(), 1);
        let method = &class_def.methods.get("oldMethod").unwrap()[0];
        assert_eq!(method.modifiers.len(), 1);
        match method.modifiers[0].0 {
            MethodModifier::Obsolete(None) => {
                let r = method.modifiers[0].1;
                assert_eq!(r.start.line, 4);
                assert_eq!(r.end.line, 4);
            }
            _ => panic!("Expected Obsolete(None) for method"),
        }
    }
}
