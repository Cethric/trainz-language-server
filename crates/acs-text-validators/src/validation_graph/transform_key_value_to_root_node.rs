use crate::{RuleNode, RulesRoot};
use std::sync::Arc;
use tracing::debug;
use trainz_ast::acs_text::{KeyValuePair, Value};

#[tracing::instrument(skip(pairs, root))]
pub(crate) fn transform_key_value_to_root_node(pairs: Vec<KeyValuePair>, root: &mut RulesRoot) {
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
