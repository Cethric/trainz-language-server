use tracing::trace;
use trainz_ast::acs_text::AcsText;
use trainz_ast::acs_text::process::process_acs_text_ast;
use trainz_parser::acs_text::parse_acs_text;

#[tracing::instrument(skip(source))]
pub(crate) fn parse_source(source: &str) -> anyhow::Result<AcsText> {
    trace!("Parsing source: {:?}", source);
    let pairs = parse_acs_text(&source)?;
    let processed = process_acs_text_ast(pairs, &source);

    Ok(processed)
}
