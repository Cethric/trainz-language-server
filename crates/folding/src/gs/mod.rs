use crate::util::collect_include_folding_ranges;
use tower_lsp_server::ls_types::FoldingRange;
use trainz_ast::gs::program::Program;

pub mod class;
pub mod stmt;
#[cfg(test)]
mod tests;

use rayon::prelude::*;

#[tracing::instrument(skip(program))]
pub fn trainz_folding_range(program: &Program) -> Vec<FoldingRange> {
    let mut result = vec![];

    collect_include_folding_ranges(&program.includes, &mut result);

    let class_folding: Vec<FoldingRange> = program
        .classes
        .par_iter()
        .flat_map(|(_, class)| {
            let mut class_result = vec![];
            class::collect_class_folding_ranges(class, &mut class_result);
            class_result
        })
        .collect();

    result.extend(class_folding);

    result
}
