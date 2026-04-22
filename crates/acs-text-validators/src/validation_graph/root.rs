use crate::validation_graph::node::RuleNode;
use anyhow::Result;
use rayon::prelude::*;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Weak};
use tracing::{trace, warn};

#[derive(Debug, Clone)]
pub struct RulesRoot {
    rules: HashMap<String, Arc<RuleNode>>,
    top_level_nodes: Vec<Weak<RuleNode>>,
}

impl RulesRoot {
    #[tracing::instrument(skip(self, node))]
    pub(crate) fn add_rule(&mut self, node: Arc<RuleNode>) {
        if node.is_top_level() {
            self.top_level_nodes.push(Arc::downgrade(&node));
        }
        if node.is_container() {
            let name = node.name();
            if let Some(evicted) = self.rules.insert(name.clone(), node) {
                warn!("Rule with name {} already exists: {:?}", name, evicted);
            }
        }
    }

    #[tracing::instrument(skip(self))]
    pub(crate) fn update_inheritance(&mut self) {
        self.rules.par_iter().for_each(|(_, rule)| {
            let ptr = Arc::as_ptr(rule) as *mut RuleNode;
            RuleNode::update_inheritance(unsafe { &mut *ptr }, self);
        });
    }

    #[tracing::instrument(skip(self, validation_path))]
    pub(crate) async fn update_sources(&mut self, validation_path: &Path) -> Result<()> {
        for (name, rule) in &self.rules {
            trace!("Updating sources for rule: {:?}", name);
            let ptr = Arc::as_ptr(rule) as *mut RuleNode;
            let result = RuleNode::update_sources(unsafe { &mut *ptr }, validation_path).await;
            if let Err(err) = result {
                warn!(
                    "Unable to update sources for rule: {:?}, error: {:?}",
                    name, err
                );
            }
        }

        Ok(())
    }

    #[tracing::instrument(skip(self, name))]
    pub fn get_rule(&self, name: &str) -> Option<Arc<RuleNode>> {
        self.rules.get(name).cloned()
    }

    #[tracing::instrument(skip(self))]
    pub fn get_top_level_nodes(&self) -> Vec<Weak<RuleNode>> {
        self.top_level_nodes.clone()
    }
}

impl RulesRoot {
    pub fn new(
        rules: HashMap<String, Arc<RuleNode>>,
        top_level_nodes: Vec<Weak<RuleNode>>,
    ) -> Self {
        Self {
            rules,
            top_level_nodes,
        }
    }
}
