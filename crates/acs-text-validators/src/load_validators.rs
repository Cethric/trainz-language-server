use crate::RulesRoot;
use crate::parse_file::parse_file;
use crate::validation_graph::transform_key_value_to_root_node::transform_key_value_to_root_node;
use anyhow::{Error, Result};
use std::collections::HashMap;
use std::path::Path;
use tracing::{debug, warn};

#[tracing::instrument(skip(validation_path, extensions_overrides_path))]
pub async fn load_validators(
    validation_path: &Path,
    extensions_overrides_path: Option<&Path>,
) -> Result<RulesRoot> {
    debug!("Loading Validation Rules from {:?}", validation_path);

    if !validation_path.exists() || !validation_path.is_dir() {
        return Err(Error::msg(format!(
            "Validation path does not exist or is not a directory: {:?}",
            validation_path
        )));
    }

    let kind_txt_path = validation_path.join("kind.txt");
    let inheritance_txt_path = validation_path.join("inheritance.txt");
    let container_txt_path = validation_path.join("container.txt");

    if !kind_txt_path.exists()
        || !kind_txt_path.is_file()
        || !inheritance_txt_path.exists()
        || !inheritance_txt_path.is_file()
        || !container_txt_path.exists()
        || !container_txt_path.is_file()
    {
        warn!("Unable to find validation rules");

        return Err(Error::msg(format!(
            "Unable to find validation rules at {:?}",
            validation_path
        )));
    }

    let (kind, inheritance, container) = tokio::try_join!(
        parse_file(&kind_txt_path),
        parse_file(&inheritance_txt_path),
        parse_file(&container_txt_path),
    )?;

    let mut root = RulesRoot::new(HashMap::new(), vec![]);

    transform_key_value_to_root_node(kind.key_value_pairs, &mut root);
    transform_key_value_to_root_node(inheritance.key_value_pairs, &mut root);
    transform_key_value_to_root_node(container.key_value_pairs, &mut root);

    if let Some(extensions_overrides_path) = extensions_overrides_path {
        debug!(
            "TODO Load extensions overrides from {:?}",
            extensions_overrides_path
        );
    }

    root.update_inheritance();

    root.update_sources(validation_path).await?;

    Ok(root)
}
