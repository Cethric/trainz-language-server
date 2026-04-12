use rayon::prelude::*;
use tower_lsp_server::ls_types::{DocumentSymbol, SymbolKind};
use trainz_ast::gs::Include;
use trainz_common::range::clamp_range;

pub mod class;
pub mod expr;
pub mod stmt;
#[cfg(test)]
mod tests;
mod util;

pub(crate) use class::process_class_symbol;
use trainz_ast::gs::program::Program;

#[allow(deprecated)]
#[tracing::instrument]
pub(crate) fn process_include_symbol(include: &Include) -> DocumentSymbol {
    DocumentSymbol {
        name: include.name.clone(),
        detail: None,
        kind: SymbolKind::MODULE,
        tags: None,
        deprecated: None,
        range: include.range,
        selection_range: clamp_range(&include.range, include.path_range.unwrap_or(include.range)),
        children: None,
    }
}

#[allow(deprecated)]
#[tracing::instrument]
pub fn trainz_symboliser(program: &Program) -> Vec<DocumentSymbol> {
    let mut symbols: Vec<DocumentSymbol> = program
        .includes
        .par_iter()
        .map(process_include_symbol)
        .collect();
    let class_symbols: Vec<DocumentSymbol> = program
        .classes
        .par_iter()
        .map(|(_, class)| process_class_symbol(class))
        .collect();
    symbols.extend(class_symbols);

    symbols.sort_by_key(|s| (s.range.start, s.selection_range.start));

    symbols
}
