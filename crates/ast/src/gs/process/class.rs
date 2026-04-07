use crate::gs::process::expr::process_expr;
use crate::gs::process::helpers::{process_identifier, process_type, process_type_or_void};
use crate::gs::process::stmt::process_statements;
use crate::gs::{
    Block, ClassDef, ClassModifier, FieldDef, FieldModifier, Identifier, MethodDef, MethodModifier,
    NativeMethodDef, Param,
};
use gs_parser::gs::grammar::Rule;
use gs_util::range::pair_to_range;
use log::trace;
use pest::iterators::Pair;
use tower_lsp_server::ls_types::Range;

pub fn process_class_definition(class_definition: Pair<Rule>) -> Option<ClassDef> {
    let range = pair_to_range(&class_definition);
    let inner = class_definition.into_inner();

    let mut class_def = ClassDef {
        modifiers: vec![],
        keyword_class_range: Range::default(),
        name: Identifier {
            name: "".to_string(),
            range: Range::default(),
        },
        keyword_is_class_range: None,
        superclasses: vec![],
        fields: vec![],
        methods: vec![],
        native_methods: vec![],
        body_range: Range::default(),
        range,
    };

    process_class_inner(inner, &mut class_def);

    Some(class_def)
}

fn process_class_inner(inner: pest::iterators::Pairs<Rule>, class_def: &mut ClassDef) {
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
                process_class_inner(pair.into_inner(), class_def);
            }
            Rule::class_member => {
                class_def.fields.push(process_field_definition(pair));
            }
            Rule::class_method => {
                class_def.methods.push(process_method_definition(pair));
            }
            Rule::class_native_method => {
                class_def
                    .native_methods
                    .push(process_native_method_definition(pair));
            }
            _ => {}
        }
    }
}

fn process_field_definition(pair: Pair<Rule>) -> FieldDef {
    let range = pair_to_range(&pair);
    let mut inner = pair.into_inner();
    let modifiers_pair = inner.next().unwrap();
    let modifiers = modifiers_pair
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

    FieldDef {
        modifiers,
        ty,
        names,
        initializers,
        range,
    }
}

fn process_method_definition(pair: Pair<Rule>) -> MethodDef {
    let range = pair_to_range(&pair);
    let mut inner = pair.into_inner();
    let modifiers = process_method_modifiers(inner.next().unwrap());
    let return_type = process_type_or_void(inner.next().unwrap());
    let name = process_identifier(inner.next().unwrap());
    let params = process_params(inner.next().unwrap());
    let body_pair = inner.next().unwrap();
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
                statements.extend(process_statements(pair));
            }
        }
        statements
    } else {
        process_statements(body_pair)
    };

    let body = Block {
        statements,
        range: body_range,
    };

    MethodDef {
        modifiers,
        return_type,
        name,
        params,
        body,
        range,
    }
}

fn process_native_method_definition(pair: Pair<Rule>) -> NativeMethodDef {
    let range = pair_to_range(&pair);
    let mut inner = pair.into_inner();
    let modifiers = process_method_modifiers(inner.next().unwrap());
    let return_type = process_type_or_void(inner.next().unwrap());
    let name = process_identifier(inner.next().unwrap());
    let params = process_params(inner.next().unwrap());

    NativeMethodDef {
        modifiers,
        is_native: true,
        return_type,
        name,
        params,
        range,
    }
}

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

fn process_params(pair: Pair<Rule>) -> Vec<Param> {
    let mut params = vec![];
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
                        params.push(Param { ty, name, range: r });
                    }
                }
            }
            Rule::line_comment | Rule::block_comment => {}
            _ => {}
        }
    }
    params
}

#[cfg(test)]
mod tests {
    use super::*;
    use gs_parser::gs::grammar::Rule;

    #[test]
    fn test_obsolete_modifiers() {
        let code = "obsolete class OldClass {\n    obsolete(123) int oldField;\n    obsolete void oldMethod() {}\n};\n";
        let pairs = gs_parser::gs::parse(code).unwrap();

        let class_pair = pairs
            .into_iter()
            .find(|p| p.as_rule() == Rule::class_definition)
            .unwrap();

        let class_def = process_class_definition(class_pair).unwrap();

        assert_eq!(class_def.modifiers.len(), 1);
        match class_def.modifiers[0].0 {
            ClassModifier::Obsolete(None) => {}
            _ => panic!("Expected Obsolete(None) for class"),
        }

        assert_eq!(class_def.fields.len(), 1);
        let field = &class_def.fields[0];
        assert_eq!(field.modifiers.len(), 1);
        match field.modifiers[0].0 {
            FieldModifier::Obsolete(Some(123)) => {}
            _ => panic!("Expected Obsolete(Some(123)) for field"),
        }

        assert_eq!(class_def.methods.len(), 1);
        let method = &class_def.methods[0];
        assert_eq!(method.modifiers.len(), 1);
        match method.modifiers[0].0 {
            MethodModifier::Obsolete(None) => {}
            _ => panic!("Expected Obsolete(None) for method"),
        }
    }
}
