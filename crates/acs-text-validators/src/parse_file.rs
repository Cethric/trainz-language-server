use crate::{parse_source, util};
use std::path::Path;
use tracing::{debug, trace};
use trainz_ast::acs_text::AcsText;

#[tracing::instrument(skip(path))]
pub(crate) async fn parse_file(path: &Path) -> anyhow::Result<AcsText> {
    debug!("Parsing {:?}", path);
    let data = tokio::fs::read(path).await?;
    let source = String::from_utf8_lossy(&data);
    let processed = parse_source::parse_source(&source)?;

    trace!("Parsed {:?} - {:?}", path, processed);
    Ok(processed)
}
