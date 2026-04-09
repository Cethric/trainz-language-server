use crate::util::collect_include_folding_ranges;
use tower_lsp_server::ls_types::FoldingRange;
use trainz_ast::gs::program::Program;

pub mod class;
pub mod stmt;
#[cfg(test)]
mod tests;

pub fn trainz_folding_range(program: &Program) -> Vec<FoldingRange> {
    let mut result = vec![];

    collect_include_folding_ranges(&program.includes, &mut result);

    for class in &program.classes {
        class::collect_class_folding_ranges(class, &mut result);
    }

    result
}
