use crate::{RuleNode, RulesRoot};
use std::sync::Arc;
use tracing::debug;
use trainz_ast::acs_text::{KeyValuePair, Value};

/// Transforms a vector of `KeyValuePair` into a `RulesRoot` node structure.
///
/// # Arguments
///
/// * `pairs`: A vector of `KeyValuePair` to transform.
/// * `root`: The `RulesRoot` to add the transformed nodes to.
///
/// # Example
///
/// ```
/// # use trainz_acs_text_validators::validation_graph::root::RulesRoot;
/// # use trainz_acs_text_validators::validation_graph::transform_key_value_to_root_node::transform_key_value_to_root_node;
/// # use std::collections::HashMap;
/// # let mut root = RulesRoot::new(HashMap::new(), vec![]);
/// # transform_key_value_to_root_node(vec![], &mut root);
/// ```
#[tracing::instrument(skip(pairs, root))]
pub fn transform_key_value_to_root_node(pairs: Vec<KeyValuePair>, root: &mut RulesRoot) {
    for kv in pairs {
        if let Some(Value::Container(values, _, _)) = &kv.value {
            let node = Arc::new_cyclic(|me| {
                RuleNode::new(me.clone(), root, kv.key.clone(), values, None, None)
            });
            root.add_rule(node);
        } else {
            debug!("Skipping {:?} - {:?}", kv.key, kv.value);
        }
    }
}
