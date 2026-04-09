use log::debug;
use rayon::iter::IntoParallelRefIterator;
use rayon::iter::ParallelIterator;
use std::str::FromStr;
use tower_lsp_server::ls_types::{
    CodeDescription, Diagnostic, DiagnosticSeverity, NumberOrString, Uri,
};
use trainz_ast::gs::program::Program;

pub fn trainz_diagnostics(program: &Program) -> Vec<Diagnostic> {
    debug!("Include paths: {:?}", program.includes);
    program
        .includes
        .par_iter()
        .filter(|include| include.path.is_none())
        .map(|include| {
            debug!("Include not found: {:?}", include);
            Diagnostic {
                range: include.range,
                severity: Some(DiagnosticSeverity::WARNING),
                code: Some(NumberOrString::Number(0)),
                code_description: if let Ok(href) = Uri::from_str(
                    "https://online.ts2009.com/mediaWiki/index.php/TrainzScript_Keywords#include",
                ) {
                    Some(CodeDescription { href })
                } else {
                    None
                },
                source: Some(String::from("game-script lsp")),
                message: format!("File not found: {}", include.name),
                related_information: None,
                tags: None,
                data: None,
            }
        })
        .collect::<Vec<Diagnostic>>()
}
