use crate::parse_as_string;
use crate::text_util::get_kind_from_text;
use crate::validation_graph::node::{RuleNode, RuleNodeKind};
use crate::validation_graph::value::RuleNodeValue;
use anyhow::Result;
use rayon::prelude::*;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Weak};
use tracing::{debug, trace, warn};
use trainz_ast::acs_text::{AcsText, KeyValuePair, Value};

#[derive(Debug, Clone)]
pub struct RulesRoot {
    rules: HashMap<String, Arc<RuleNode>>,
    top_level_nodes: Vec<Weak<RuleNode>>,
}

impl RulesRoot {
    /// Retrieves a rule node by its path in the `AcsText`.
    ///
    /// # Arguments
    ///
    /// * `acs_text`: The `AcsText` to search in.
    /// * `path`: The path of `KeyValuePair`s to the node.
    ///
    /// # Returns
    ///
    /// An `Option<Arc<RuleNode>>` containing the node, or `None` if not found.
    ///
    /// # Example
    ///
    /// ```
    /// # use trainz_acs_text_validators::validation_graph::root::RulesRoot;
    /// # // Assuming a valid RulesRoot instance 'root' and AcsText instance 'acs_text'
    /// # // let node = root.get_node_by_path(Some(&acs_text), &[]);
    /// ```
    #[tracing::instrument(skip(self, acs_text, path))]
    pub fn get_node_by_path(
        &self,
        acs_text: Option<&AcsText>,
        path: &[&KeyValuePair],
    ) -> Option<Arc<RuleNode>> {
        if path.is_empty() {
            return None;
        }

        let kind = acs_text.and_then(|a| get_kind_from_text(a).map(|(k, _, _)| k));

        let (current_node, traversal_path) = if let Some(kind) = kind {
            let node = self.find_node_by_name(&kind)?;
            if path[0].key.eq_ignore_ascii_case(&kind) {
                (node, &path[1..])
            } else {
                (node, path)
            }
        } else {
            (self.get_rule(&path[0].key)?, &path[1..])
        };

        let mut current_node = current_node;
        for name in traversal_path {
            current_node = self.get_next_node(&current_node, name)?;
        }
        Some(current_node)
    }

    #[tracing::instrument(skip(self, current_node, name))]
    fn get_next_node(
        &self,
        current_node: &Arc<RuleNode>,
        name: &KeyValuePair,
    ) -> Option<Arc<RuleNode>> {
        if let Some(current_kind) = current_node.kind() {
            match current_kind {
                RuleNodeKind::Value(_) => Some(current_node.clone()),
                RuleNodeKind::Element(element) => {
                    if let Some(node) = element.element()
                        && let Some(node) = node.upgrade()
                    {
                        debug!("Found element: {}", node.name());
                        Some(node)
                    } else {
                        None
                    }
                }
                RuleNodeKind::Structure(structure) => {
                    // Check possibilities
                    if let Some(node) = structure.get_possibility(&name.key) {
                        return node.upgrade();
                    }
                    // Check tag array - exact match
                    if let Some(tag_array) = structure.tag_array()
                        && let Some(node) = tag_array.upgrade()
                    {
                        return Some(node);
                    }
                    // Check array elements
                    let mut matched_node = None;
                    for (element_name, node_weak) in structure.array_elements_as_vec() {
                        if element_name.eq_ignore_ascii_case(&name.key) {
                            matched_node = node_weak.upgrade();
                            break;
                        }
                    }

                    if let Some(node) = matched_node {
                        return Some(node);
                    }

                    // Fallback: If it's an array, return the first element
                    if let Some(Value::Container(container, _, _)) = &name.value
                        && let Some(container) = container
                            .par_iter()
                            .find_first(|kv| kv.key.eq_ignore_ascii_case("kind"))
                        && let Some(kind) = parse_as_string(container)
                    {
                        let node = structure
                            .array_elements()
                            .par_iter()
                            .filter_map(|node| node.upgrade())
                            .find_first(|node| {
                                if let Some(RuleNodeKind::Value(value)) = node.kind()
                                    && let RuleNodeValue::String(value) = value
                                    && let Some(default) = value.default()
                                    && default.eq_ignore_ascii_case(&kind)
                                {
                                    true
                                } else {
                                    false
                                }
                            });
                        if let Some(node) = node {
                            return Some(node);
                        } else {
                            debug!(
                                "Could not find kind {} in array element: {:?}",
                                kind, name.key
                            )
                        }
                    } else {
                        debug!("Could not find kind in array element: {:?}", name.key)
                    }

                    if let Some(first_node) =
                        structure.array_elements().first().and_then(|n| n.upgrade())
                    {
                        return Some(first_node);
                    }

                    // Fallback: Arbitrary key match
                    if let Some(tag_array) = structure.tag_array()
                        && let Some(node) = tag_array.upgrade()
                    {
                        return Some(node);
                    }
                    self.get_rule(&name.key)
                }
            }
        } else {
            None
        }
    }

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

    /// Retrieves a rule node from a path, along with inheritance information.
    ///
    /// # Arguments
    ///
    /// * `kind`: The kind of rule node to look for.
    /// * `path`: The path of `KeyValuePair`s to the node.
    ///
    /// # Returns
    ///
    /// `Option<(bool, Vec<Arc<RuleNode>>)>` containing success status and inherited rule nodes, or `None` if not found.
    ///
    /// # Example
    ///
    /// ```
    /// # use trainz_acs_text_validators::validation_graph::root::RulesRoot;
    /// # // Assuming a valid RulesRoot instance 'root'
    /// # // let result = root.get_rule_from_path("kind", &[]);
    /// ```
    #[tracing::instrument(skip(self, kind, path))]
    pub fn get_rule_from_path(
        &self,
        kind: &str,
        path: &[KeyValuePair],
    ) -> Option<(bool, Vec<Arc<RuleNode>>)> {
        #[tracing::instrument(skip(node))]
        fn node_inheritance(node: &Arc<RuleNode>) -> Vec<Arc<RuleNode>> {
            node.inheritance()
                .par_iter()
                .filter_map(|node| {
                    if let Some(RuleNodeKind::Structure(structure)) = node.kind() {
                        Some(
                            structure
                                .possibilities()
                                .par_iter()
                                .filter_map(|(_, node)| node.upgrade())
                                .map(|node| node.inheritance())
                                .flatten()
                                .collect(),
                        )
                    } else if let Some(RuleNodeKind::Element(element)) = node.kind()
                        && let Some(element_node) = element.element()
                        && let Some(element_node) = element_node.upgrade()
                    {
                        Some(node_inheritance(&element_node))
                    } else if let Some(RuleNodeKind::Value(_)) = node.kind() {
                        Some(vec![node.clone()])
                    } else {
                        None
                    }
                })
                .flatten()
                .collect::<Vec<Arc<RuleNode>>>()
        }

        #[tracing::instrument(skip(node, kind, search_path))]
        fn process_kind(
            node: &Arc<RuleNode>,
            kind: &RuleNodeKind,
            search_path: &[KeyValuePair],
            matched: &mut Vec<Arc<RuleNode>>,
        ) -> (Option<Vec<Arc<RuleNode>>>, Option<Vec<KeyValuePair>>) {
            debug!("Found kind: {:?}", kind);
            match kind {
                RuleNodeKind::Value(_) => (Some(node_inheritance(node)), Some(vec![])),
                RuleNodeKind::Element(element) => {
                    if let Some(element) = element.element()
                        && let Some(element) = element.upgrade()
                    {
                        if let Some(kind) = element.kind() {
                            debug!("Found kind: {:?}", kind);
                            process_kind(&element, kind, &search_path, matched)
                        } else {
                            (Some(node_inheritance(&element)), None)
                        }
                    } else {
                        (None, None)
                    }
                }
                RuleNodeKind::Structure(structure) => {
                    if !structure.array_elements().is_empty() || structure.tag_array().is_some() {
                        if let Some(tag_array) = structure.tag_array()
                            && let Some(tag_array) = tag_array.upgrade()
                        {
                            debug!("Found tag array: {:?}", tag_array);
                            matched.par_extend(node_inheritance(&tag_array));
                            (
                                Some(node_inheritance(&tag_array)),
                                if let Some((dropped, paths)) = search_path.split_first() {
                                    debug!("Dropped path {}", dropped.key);
                                    Some(
                                        paths
                                            .iter()
                                            .map(|x| x.clone())
                                            .collect::<Vec<KeyValuePair>>(),
                                    )
                                } else {
                                    None
                                },
                            )
                        } else {
                            let array_elements = structure.array_elements();
                            debug!("Found array elements: {:?}", array_elements);
                            (
                                Some(
                                    array_elements
                                        .par_iter()
                                        .filter_map(|node| {
                                            node.upgrade().map(|node| node_inheritance(&node))
                                        })
                                        .flatten()
                                        .collect::<Vec<Arc<RuleNode>>>(),
                                ),
                                if let Some((dropped, paths)) = search_path.split_first() {
                                    debug!("Dropped path {}", dropped.key);
                                    Some(
                                        paths
                                            .iter()
                                            .map(|x| x.clone())
                                            .collect::<Vec<KeyValuePair>>(),
                                    )
                                } else {
                                    None
                                },
                            )
                        }
                    } else {
                        debug!("Found structure: {:?}", structure);
                        (
                            Some(
                                structure
                                    .possibilities()
                                    .values()
                                    .filter_map(|possibility| possibility.upgrade())
                                    .collect::<Vec<Arc<RuleNode>>>(),
                            ),
                            None,
                        )
                    }
                }
            }
        }

        debug!(
            "Querying container path {} > {}",
            kind,
            path.iter()
                .map(|p| p.key.to_string())
                .collect::<Vec<String>>()
                .join(" > "),
        );

        if let Some(kind_node) = self.rules.get(kind) {
            if path.is_empty() {
                Some((true, vec![kind_node.clone()]))
            } else {
                let mut nodes = node_inheritance(kind_node);
                let mut matched: Vec<Arc<RuleNode>> = Vec::new();
                let mut search_path: Vec<KeyValuePair> = path.to_owned();

                while let Some((current, next)) = search_path.split_first() {
                    debug!(
                        "Searching for: {} in [ {} ]",
                        current.key,
                        nodes
                            .par_iter()
                            .map(|n| n.name())
                            .collect::<Vec<String>>()
                            .join(", ")
                    );

                    if let Some(found) = nodes
                        .clone()
                        .par_iter()
                        .find_first(|node| node.near_name(&current.key))
                    {
                        debug!("Found item {}", current.key);
                        if let Some(kind) = found.kind() {
                            debug!("Found kind: {:?}", kind);
                            let (processed_nodes, processed_search) =
                                process_kind(found, kind, next, &mut matched);
                            if let Some(processed_nodes) = processed_nodes {
                                nodes = processed_nodes.clone();
                            } else {
                                nodes = node_inheritance(found);
                            }
                            if let Some(processed_search) = processed_search {
                                search_path = processed_search;
                            } else {
                                search_path = next.to_vec();
                            }
                        }
                        matched.push(found.clone());
                    } else {
                        debug!("Could not find item {}", current.key);
                        break;
                    }
                }

                if matched.is_empty() {
                    Some((false, nodes))
                } else {
                    Some((true, matched))
                }
            }
        } else {
            None
        }
    }

    /// Retrieves a rule node by its name.
    ///
    /// # Arguments
    ///
    /// * `name`: The name of the rule to retrieve.
    ///
    /// # Returns
    ///
    /// An `Option<Arc<RuleNode>>` containing the rule node, or `None` if not found.
    ///
    /// # Example
    ///
    /// ```
    /// # use trainz_acs_text_validators::validation_graph::root::RulesRoot;
    /// # // Assuming a valid RulesRoot instance 'root'
    /// # // let rule = root.get_rule("name");
    /// ```
    #[tracing::instrument(skip(self, name))]
    pub fn get_rule(&self, name: &str) -> Option<Arc<RuleNode>> {
        self.rules.get(name).cloned()
    }

    /// Finds a node by its name.
    ///
    /// # Arguments
    ///
    /// * `name`: The name of the node to find.
    ///
    /// # Returns
    ///
    /// An `Option<Arc<RuleNode>>` containing the node, or `None` if not found.
    ///
    /// # Example
    ///
    /// ```
    /// # use trainz_acs_text_validators::validation_graph::root::RulesRoot;
    /// # // Assuming a valid RulesRoot instance 'root'
    /// # // let node = root.find_node_by_name("name");
    /// ```
    #[tracing::instrument(skip(self, name))]
    pub fn find_node_by_name(&self, name: &str) -> Option<Arc<RuleNode>> {
        self.get_rule(name).or_else(|| {
            self.top_level_nodes
                .iter()
                .filter_map(|n| n.upgrade())
                .find(|n| n.matches_name(name))
        })
    }

    /// Returns the top-level nodes.
    ///
    /// # Returns
    ///
    /// A `Vec<Weak<RuleNode>>` containing the top-level nodes.
    ///
    /// # Example
    ///
    /// ```
    /// # use trainz_acs_text_validators::validation_graph::root::RulesRoot;
    /// # // Assuming a valid RulesRoot instance 'root' exists
    /// # // let nodes = root.get_top_level_nodes();
    /// ```
    #[tracing::instrument(skip(self))]
    pub fn get_top_level_nodes(&self) -> Vec<Weak<RuleNode>> {
        self.top_level_nodes.clone()
    }
}

impl RulesRoot {
    /// Creates a new `RulesRoot`.
    ///
    /// # Arguments
    ///
    /// * `rules`: A map of rule names to rule nodes.
    /// * `top_level_nodes`: A vector of weak references to top-level rule nodes.
    ///
    /// # Returns
    ///
    /// A new `RulesRoot` instance.
    ///
    /// # Example
    ///
    /// ```
    /// # use trainz_acs_text_validators::validation_graph::root::RulesRoot;
    /// # use std::collections::HashMap;
    /// # let root = RulesRoot::new(HashMap::new(), vec![]);
    /// ```
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
