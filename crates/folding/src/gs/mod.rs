use crate::util::collect_include_folding_ranges;
use gs_ast::gs::Program;
use tower_lsp_server::ls_types::FoldingRange;

pub mod class;
pub mod stmt;
#[cfg(test)]
mod tests;

pub fn gs_folding_range(program: &Program) -> Vec<FoldingRange> {
    let mut result = vec![];

    collect_include_folding_ranges(&program.includes, &mut result);

    for class in &program.classes {
        class::collect_class_folding_ranges(class, &mut result);
    }

    result
}
