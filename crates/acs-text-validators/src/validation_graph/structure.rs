use crate::parse_as_string;
use crate::util::parse_as_bool;
use crate::validation_graph::node::RuleNode;
use crate::validation_graph::root::RulesRoot;
use anyhow::Result;
use async_recursion::async_recursion;
use rayon::prelude::*;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Weak};
use tracing::{debug, warn};
use trainz_ast::acs_text::{KeyValuePair, Value};

#[derive(Debug, Clone)]
pub struct RuleNodeStructure {
    unique: bool,
    values: HashMap<String, Arc<RuleNode>>,
    possibilities: HashMap<String, Weak<RuleNode>>,
    tag_array: Option<Weak<RuleNode>>,
    is_array: bool,
    array_elements_map: HashMap<String, String>,
    array_elements: Vec<(String, Weak<RuleNode>)>,
}

impl RuleNodeStructure {
    #[tracing::instrument(skip(self))]
    pub fn unique(&self) -> bool {
        self.unique
    }

    #[tracing::instrument(skip(self))]
    pub fn is_array(&self) -> bool {
        self.is_array
    }

    #[tracing::instrument(skip(self))]
    pub fn possibilities(&self) -> &HashMap<String, Weak<RuleNode>> {
        &self.possibilities
    }

    #[tracing::instrument(skip(self, name))]
    pub fn get_possibility(&self, name: &str) -> Option<Weak<RuleNode>> {
        self.possibilities.get(name).cloned()
    }

    #[tracing::instrument(skip(self))]
    pub fn tag_array(&self) -> Option<Weak<RuleNode>> {
        self.tag_array.clone()
    }

    #[tracing::instrument(skip(self, key))]
    pub fn get_element_by_key(&self, key: &str) -> Option<Weak<RuleNode>> {
        self.array_elements
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, node)| node.clone())
    }

    #[tracing::instrument(skip(self))]
    pub fn array_elements(&self) -> Vec<Weak<RuleNode>> {
        self.array_elements
            .iter()
            .map(|(_, node)| node.clone())
            .collect()
    }

    #[tracing::instrument(skip(self))]
    pub fn array_elements_as_vec(&self) -> &Vec<(String, Weak<RuleNode>)> {
        &self.array_elements
    }

    #[tracing::instrument(skip(self))]
    pub fn array_elements_map(&self) -> &HashMap<String, String> {
        &self.array_elements_map
    }

    #[tracing::instrument(skip(self))]
    pub fn details(&self) -> Option<String> {
        None
    }

    #[tracing::instrument(skip(self))]
    pub fn description(&self) -> Option<String> {
        if let Some(tag_array) = &self.tag_array {
            if let Some(tag_array) = tag_array.upgrade()
                && let Some(tag_array_description) = tag_array.description()
            {
                Some(format!("TagArray({})", tag_array_description))
            } else {
                Some(String::from("TagArray"))
            }
        } else if self.is_array || !self.array_elements_map.is_empty() {
            Some(format!(
                "Array({})",
                self.array_elements_map
                    .values()
                    .cloned()
                    .collect::<Vec<String>>()
                    .join(" | ")
            ))
        } else {
            Some(String::from("Structure"))
        }
    }

    #[tracing::instrument(skip(self, _trainz_version))]
    pub fn documentation(&self, _trainz_version: &f64) -> Option<String> {
        // TODO - implement documentation
        None
    }
}

impl RuleNodeStructure {
    #[tracing::instrument(skip(self, root))]
    pub(crate) fn update_inheritance(&mut self, root: &RulesRoot) {
        self.values.par_iter_mut().for_each(|(_, node)| {
            let ptr = Arc::as_ptr(node) as *mut RuleNode;
            unsafe { &mut *ptr }.update_inheritance(root);
        });

        let mut elements: Vec<_> = self.array_elements_map.iter().collect();
        elements.sort_by_key(|(key, _)| *key);
        self.array_elements = elements
            .into_iter()
            .filter_map(|(key, name)| {
                if let Some(rule) = root.get_rule(name) {
                    Some((key.clone(), Arc::downgrade(&rule)))
                } else {
                    warn!("Array element not found: {}", name);
                    None
                }
            })
            .collect::<Vec<(String, Weak<RuleNode>)>>();

        if self.array_elements.len() != self.array_elements_map.len() {
            warn!(
                "Unable to find all array elements: {}",
                self.array_elements_map
                    .values()
                    .cloned()
                    .collect::<Vec<String>>()
                    .join(",")
            )
        }
    }

    #[async_recursion]
    #[tracing::instrument(skip(self, validation_path))]
    pub(crate) async fn update_sources(&mut self, validation_path: &Path) -> Result<()> {
        for element in self.values.values() {
            let ptr = Arc::as_ptr(element) as *mut RuleNode;
            let result = RuleNode::update_sources(unsafe { &mut *ptr }, validation_path).await;
            if let Err(err) = result {
                warn!(
                    "Unable to update sources for rule: {:?}, error: {:?}",
                    element.name(),
                    err
                );
            }
        }
        Ok(())
    }
}

pub(crate) const SUB_POSSIBILITIES_KEY: &str = "SubPossibilities";
pub(crate) const TAG_ARRAY_KEY: &str = "TagArray";
pub(crate) const ARRAY_ELEMENT_KEY: &str = "Array-Element";

impl RuleNodeStructure {
    #[tracing::instrument(skip(root, entries, is_array, parent))]
    pub(crate) fn new(
        root: &mut RulesRoot,
        entries: &Vec<KeyValuePair>,
        is_array: bool,
        parent: &Option<Weak<RuleNode>>,
    ) -> Self {
        let mut unique = false;
        let mut possibilities: HashMap<String, Weak<RuleNode>> = HashMap::new();
        let mut values: HashMap<String, Arc<RuleNode>> = HashMap::new();
        let mut tag_array: Option<Weak<RuleNode>> = None;
        let mut array_elements_map: HashMap<String, String> = HashMap::new();

        for entry in entries {
            if entry.key.eq_ignore_ascii_case("unique") {
                unique = parse_as_bool(entry);
            }

            if entry.key.eq_ignore_ascii_case(SUB_POSSIBILITIES_KEY)
                && let Some(Value::Container(container_kv, _, _)) = &entry.value
            {
                for key_value_pair in container_kv {
                    if let Some(Value::Container(sub_values, _, _)) = &key_value_pair.value {
                        let node = Arc::new_cyclic(|me| {
                            RuleNode::new(
                                me.clone(),
                                root,
                                key_value_pair.key.clone(),
                                sub_values,
                                None,
                                parent.clone(),
                            )
                        });
                        possibilities.insert(key_value_pair.key.clone(), Arc::downgrade(&node));
                        values.insert(key_value_pair.key.clone(), node.clone());
                        root.add_rule(node);
                    } else {
                        warn!(
                            "Sub Possibilities value is not a container: {:?}",
                            key_value_pair.value
                        );
                    }
                }
            }

            if entry.key.eq_ignore_ascii_case("obsolete-tag")
                && let Some(Value::Container(value, _, _)) = &entry.value
            {
                for sub_entry in value {
                    if let Some(Value::Container(sub_values, _, _)) = &sub_entry.value {
                        let node = Arc::new_cyclic(|me| {
                            RuleNode::new(
                                me.clone(),
                                root,
                                sub_entry.key.clone(),
                                sub_values,
                                Some(1.0f64),
                                parent.clone(),
                            )
                        });
                        possibilities.insert(sub_entry.key.clone(), Arc::downgrade(&node));
                        values.insert(sub_entry.key.clone(), node.clone());
                        root.add_rule(node);
                    }
                }
            }

            if entry.key.eq_ignore_ascii_case(TAG_ARRAY_KEY)
                && let Some(Value::Container(sub_values, _, _)) = &entry.value
            {
                let node = Arc::new_cyclic(|me| {
                    RuleNode::new(
                        me.clone(),
                        root,
                        entry.key.clone(),
                        sub_values,
                        None,
                        parent.clone(),
                    )
                });
                debug!("TagArray found for node {}", node.name());
                tag_array = Some(Arc::downgrade(&node));
                values.insert("TagArray".to_string(), node.clone());
                root.add_rule(node);
            }

            if entry.key.eq_ignore_ascii_case(ARRAY_ELEMENT_KEY)
                && let Some(Value::Container(container_kv, _, _)) = &entry.value
            {
                for kv in container_kv {
                    if let Some(element_name) = parse_as_string(kv) {
                        array_elements_map.insert(kv.key.clone(), element_name);
                    }
                }
            }
        }

        Self {
            unique,
            possibilities,
            values,
            tag_array,
            is_array,
            array_elements_map,
            array_elements: vec![],
        }
    }
}
