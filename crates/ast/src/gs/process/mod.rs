use crate::gs::process::helpers::push_scope;
use crate::gs::program::Program;
use crate::gs::{ClassDef, Include, Scope};
use pest::iterators::Pairs;
use std::collections::HashMap;
use trainz_common::range::pos_to_range;
use trainz_parser::gs::grammar::Rule;

mod class;
mod expr;
#[cfg(test)]
mod expr_tests;
mod helpers;
mod include;
mod stmt;
#[cfg(test)]
mod tests;

#[tracing::instrument(skip(pairs, src))]
pub fn process_trainz_ast(pairs: Pairs<Rule>, src: &str) -> Program {
    let mut includes = vec![];
    let mut class_definitions = HashMap::new();
    let mut root_range = tower_lsp_server::ls_types::Range::default();
    let mut scopes: Vec<Scope> = vec![];

    if let Some(first_pair) = pairs.clone().next() {
        let first_pos = first_pair.as_span().start_pos();
        let last_pos = pairs.clone().last().unwrap().as_span().end_pos();
        root_range = pos_to_range(&first_pos, &last_pos);
    }

    let root_scope_id = push_scope(&mut scopes, None, root_range, vec![]);

    fn traverse(
        scopes: &mut Vec<Scope>,
        root_scope_id: usize,
        pairs: Pairs<Rule>,
        includes: &mut Vec<Include>,
        class_definitions: &mut HashMap<String, ClassDef>,
    ) {
        for pair in pairs {
            match pair.as_rule() {
                Rule::include_statement => {
                    if let Some(include) = include::process_include(pair) {
                        includes.push(include);
                    }
                }
                Rule::class_definition => {
                    if let Some(class_definition) =
                        class::process_class_definition(scopes, root_scope_id, pair)
                    {
                        class_definitions
                            .insert(class_definition.name.name.clone(), class_definition);
                    }
                }
                _ => {
                    traverse(
                        scopes,
                        root_scope_id,
                        pair.into_inner(),
                        includes,
                        class_definitions,
                    );
                }
            }
        }
    }

    traverse(
        &mut scopes,
        root_scope_id,
        pairs.clone(),
        &mut includes,
        &mut class_definitions,
    );

    Program {
        includes,
        classes: class_definitions,
        scopes,
        root_scope_id,
        range: root_range,
        src: src.to_string(),
    }
}
