use crate::gs::stmt::collect_block_folding_ranges;
use crate::util::add_folding_range;
use tower_lsp_server::ls_types::FoldingRange;
use trainz_ast::gs::ClassDef;

pub fn collect_class_folding_ranges(class: &ClassDef, result: &mut Vec<FoldingRange>) {
    add_folding_range(class.body_range, result, Some(String::from("{ ... }")));

    for methods in class.methods.values() {
        for method in methods {
            collect_method_folding_ranges(method, result);
        }
    }
}

fn collect_method_folding_ranges(
    method: &trainz_ast::gs::MethodDef,
    result: &mut Vec<FoldingRange>,
) {
    if let Some(body) = &method.body {
        add_folding_range(body.range, result, Some(String::from("{ ... }")));
        collect_block_folding_ranges(body, result);
    }
}
