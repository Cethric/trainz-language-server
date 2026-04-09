use crate::gs::stmt::collect_block_folding_ranges;
use crate::util::add_folding_range;
use tower_lsp_server::ls_types::FoldingRange;
use trainz_ast::gs::ClassDef;

pub fn collect_class_folding_ranges(class: &ClassDef, result: &mut Vec<FoldingRange>) {
    add_folding_range(class.body_range, result, Some(String::from("{ ... }")));

    for method in &class.methods {
        collect_method_folding_ranges(method, result);
    }
}

fn collect_method_folding_ranges(
    method: &trainz_ast::gs::MethodDef,
    result: &mut Vec<FoldingRange>,
) {
    add_folding_range(method.body.range, result, Some(String::from("{ ... }")));
    collect_block_folding_ranges(&method.body, result);
}
