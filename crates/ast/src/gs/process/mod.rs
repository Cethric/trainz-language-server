use crate::gs::{ClassDef, Include, Program};
use gs_parser::gs::grammar::Rule;
use gs_util::range::pos_to_range;
use pest::iterators::Pairs;

mod class;
mod expr;
mod helpers;
mod include;
mod stmt;
#[cfg(test)]
mod tests;

pub fn process_gs_ast(pairs: Pairs<Rule>, _src: &str) -> Program {
    let mut includes = vec![];
    let mut class_definitions = vec![];
    let mut root_range = tower_lsp_server::ls_types::Range::default();

    fn traverse(
        pairs: Pairs<Rule>,
        includes: &mut Vec<Include>,
        class_definitions: &mut Vec<ClassDef>,
    ) {
        for pair in pairs {
            match pair.as_rule() {
                Rule::include_statement => {
                    if let Some(include) = include::process_include(pair) {
                        includes.push(include);
                    }
                }
                Rule::class_definition => {
                    if let Some(class_definition) = class::process_class_definition(pair) {
                        class_definitions.push(class_definition);
                    }
                }
                _ => {
                    traverse(pair.into_inner(), includes, class_definitions);
                }
            }
        }
    }

    traverse(pairs.clone(), &mut includes, &mut class_definitions);

    if let Some(first_pair) = pairs.clone().next() {
        let first_pos = first_pair.as_span().start_pos();
        let last_pos = pairs.clone().last().unwrap().as_span().end_pos();
        root_range = pos_to_range(&first_pos, &last_pos);
    }

    Program {
        includes,
        classes: class_definitions,
        range: root_range,
    }
}
