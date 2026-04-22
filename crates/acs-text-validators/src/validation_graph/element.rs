use crate::parse_as_string;
use crate::util::parse_as_bool;
use crate::validation_graph::node::RuleNode;
use crate::validation_graph::root::RulesRoot;
use anyhow::Result;
use async_recursion::async_recursion;
use std::path::Path;
use std::sync::{Arc, Weak};
use tracing::{debug, warn};
use trainz_ast::acs_text::KeyValuePair;

#[derive(Debug, Clone)]
pub struct RuleNodeElement {
    element_name: Option<String>,
    element: Option<Weak<RuleNode>>,
    is_num_array: bool,
}

impl RuleNodeElement {
    pub fn element(&self) -> Option<Weak<RuleNode>> {
        self.element.clone()
    }

    pub fn is_num_array(&self) -> bool {
        self.is_num_array
    }
}

impl RuleNodeElement {
    #[tracing::instrument(skip(self, root))]
    pub(crate) fn update_inheritance(&mut self, root: &RulesRoot) {
        if let Some(element_name) = &self.element_name
            && let Some(node) = root.get_rule(element_name)
        {
            debug!("Found element: {} for rule: {}", node.name(), element_name);
            let weak_node = Arc::downgrade(&node);
            self.element = Some(weak_node);
        } else {
            warn!("Element not found for rule: {:?}", self.element_name);
        }
    }

    #[async_recursion]
    #[tracing::instrument(skip(self, _validation_path))]
    pub(crate) async fn update_sources(&mut self, _validation_path: &Path) -> Result<()> {
        // if let Some(element) = self.element.as_mut()
        //     && let Some(element) = Weak::upgrade(element).as_mut()
        // {
        //     let ptr = Arc::as_ptr(element) as *mut RuleNode;
        //     let result = RuleNode::update_sources(unsafe { &mut *ptr }, validation_path).await;
        //     if let Err(err) = result {
        //         warn!(
        //             "Unable to update sources for rule: {:?}, error: {:?}",
        //             element.name(),
        //             err
        //         );
        //     }
        // }
        Ok(())
    }
}

impl RuleNodeElement {
    #[tracing::instrument(skip(entries))]
    pub(crate) fn new(entries: &Vec<KeyValuePair>) -> Self {
        let mut is_num_array: bool = false;
        let mut element_name: Option<String> = None;
        for entry in entries {
            if entry.key.eq_ignore_ascii_case("numarray") {
                is_num_array = parse_as_bool(entry);
            }
            if entry.key.eq_ignore_ascii_case("element-type") {
                element_name = parse_as_string(entry);
            }
        }

        Self {
            element_name,
            element: None,
            is_num_array,
        }
    }
}
