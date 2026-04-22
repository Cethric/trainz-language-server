use crate::util::{parse_as_bool, parse_as_string, parse_as_value_is_non_zero};
use crate::validation_graph::element::RuleNodeElement;
use crate::validation_graph::root::RulesRoot;
use crate::validation_graph::structure::{
    ARRAY_ELEMENT_KEY, RuleNodeStructure, SUB_POSSIBILITIES_KEY,
};
use crate::validation_graph::validation::Validator;
use crate::validation_graph::value::RuleNodeValue;
use anyhow::Result;
use async_recursion::async_recursion;
use rayon::prelude::*;
use std::collections::HashMap;
use std::error::Error;
use std::path::Path;
use std::sync::{Arc, Weak};
use tracing::{debug, trace, warn};
use trainz_ast::acs_text::{KeyValuePair, Value};

#[derive(Debug, Clone)]
pub struct RuleNode {
    me: Weak<Self>,
    name: String,
    description: Option<String>,
    top_level: bool,
    obsolete: Option<f64>,
    compulsory: Option<f64>,
    minimum_version: Option<f64>,
    parent: Option<Weak<RuleNode>>,
    kind: Option<RuleNodeKind>,
    validators: Vec<Validator>,
    dependencies: HashMap<String, String>,
    inherited_names: Vec<String>,
    inherited_nodes: Vec<Weak<RuleNode>>,
}

#[derive(Debug, Clone)]
pub enum RuleNodeKind {
    Value(RuleNodeValue),
    Element(RuleNodeElement),
    Structure(RuleNodeStructure),
}

impl RuleNode {
    #[tracing::instrument(skip(me, root, name, entries, obsolete, parent))]
    pub(crate) fn new(
        me: Weak<Self>,
        root: &mut RulesRoot,
        name: String,
        entries: &Vec<KeyValuePair>,
        obsolete: Option<f64>,
        parent: Option<Weak<RuleNode>>,
    ) -> Self {
        let mut description: Option<String> = None;
        let mut top_level = false;
        let mut obsolete: Option<f64> = obsolete;
        let mut compulsory: Option<f64> = None;
        let mut minimum_version: Option<f64> = None;
        let mut kind: Option<RuleNodeKind> = None;
        let mut inherited_names: Vec<String> = vec![];

        for entry in entries {
            if entry.key.eq_ignore_ascii_case("description") {
                description = parse_as_string(entry);
            }

            if entry.key.eq_ignore_ascii_case("top-level") {
                top_level = parse_as_bool(entry);
            }

            if entry.key.eq_ignore_ascii_case("compulsory") {
                compulsory = parse_as_value_is_non_zero::<f64>(entry);
            }

            if entry.key.eq_ignore_ascii_case("obsolete")
                || entry.key.eq_ignore_ascii_case("obsolete-version")
            {
                obsolete = parse_as_value_is_non_zero::<f64>(entry);
            }

            if entry.key.eq_ignore_ascii_case("minimum-version") {
                minimum_version = parse_as_value_is_non_zero::<f64>(entry);
            }

            if entry.key.eq_ignore_ascii_case("kind") {
                let kind_str = parse_as_string(entry);
                if let Some(kind_str) = kind_str {
                    kind = RuleNodeKind::new(root, &name, &kind_str, entries, &parent);
                }
            }

            if entry.key.eq_ignore_ascii_case("inherit")
                && let Some(Value::Container(container, _, _)) = &entry.value
            {
                for kv in container {
                    inherited_names.push(kv.key.clone());
                }
            }
        }

        if kind.is_none() {
            if entries.par_iter().any(|entry| {
                entry.key.eq_ignore_ascii_case(ARRAY_ELEMENT_KEY)
                    || entry.key.eq_ignore_ascii_case(SUB_POSSIBILITIES_KEY)
            }) {
                kind = RuleNodeKind::new(root, &name, STRUCTURE_KEY, entries, &parent);
            } else {
                debug!("No kind specified for node: {}", name);
            }
        }

        Self {
            me,
            name,
            description,
            top_level,
            obsolete,
            compulsory,
            minimum_version,
            parent,
            kind,
            validators: vec![],
            dependencies: HashMap::new(),
            inherited_names,
            inherited_nodes: vec![],
        }
    }

    #[tracing::instrument(skip(self, root))]
    pub(crate) fn update_inheritance(&mut self, root: &RulesRoot) {
        for name in self.inherited_names.iter() {
            if let Some(node) = root.get_rule(name) {
                let weak_node = Arc::downgrade(&node);
                self.inherited_nodes.push(weak_node);
            }
        }

        if let Some(kind) = self.kind.as_mut() {
            kind.update_inheritance(root);
        }
    }

    #[async_recursion]
    #[tracing::instrument(skip(self, validation_path))]
    pub(crate) async fn update_sources(&mut self, validation_path: &Path) -> Result<()> {
        debug!("Updating sources for node: {:?}", self.name);
        if let Some(kind) = self.kind.as_mut() {
            kind.update_sources(validation_path).await?;
        }
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub fn name(&self) -> String {
        self.name.clone()
    }

    #[tracing::instrument(skip(self))]
    pub fn kind(&self) -> Option<&RuleNodeKind> {
        self.kind.as_ref()
    }

    #[tracing::instrument(skip(self))]
    pub fn is_top_level(&self) -> bool {
        self.top_level
    }

    #[tracing::instrument(skip(self))]
    pub fn is_container(&self) -> bool {
        if let Some(kind) = &self.kind {
            matches!(kind, RuleNodeKind::Structure(_))
        } else {
            false
        }
    }

    #[tracing::instrument(skip(self))]
    pub fn is_element(&self) -> bool {
        if let Some(kind) = &self.kind {
            matches!(kind, RuleNodeKind::Element(_))
        } else {
            false
        }
    }

    #[tracing::instrument(skip(self, name))]
    pub fn matches_name(&self, name: &str) -> bool {
        self.name.eq_ignore_ascii_case(name)
    }

    #[tracing::instrument(skip(self, trainz_build))]
    pub fn is_compulsory(&self, trainz_build: f64) -> bool {
        if let Some(compulsory) = self.compulsory {
            trainz_build >= compulsory
        } else {
            false
        }
    }

    #[tracing::instrument(skip(self, trainz_build))]
    pub fn is_obsolete(&self, trainz_build: f64) -> bool {
        if let Some(obsolete) = self.obsolete {
            trainz_build >= obsolete
        } else {
            false
        }
    }

    #[tracing::instrument(skip(self, trainz_build))]
    pub fn is_supported(&self, trainz_build: f64) -> bool {
        if let Some(minimum_version) = self.minimum_version {
            trainz_build >= minimum_version
        } else {
            true
        }
    }

    #[tracing::instrument(skip(self))]
    pub fn obsolete_since(&self) -> Option<f64> {
        self.obsolete
    }

    #[tracing::instrument(skip(self))]
    pub fn inheritance(&self) -> Vec<Arc<RuleNode>> {
        trace!("Flattening inheritance for {:?}", self);
        let mut inheritance: Vec<Arc<RuleNode>> = self
            .inherited_nodes
            .par_iter()
            .filter_map(|node| node.upgrade())
            .map(|node| node.inheritance())
            .flatten()
            .collect();
        if let Some(me) = self.me.upgrade() {
            inheritance.push(me);
        } else {
            warn!(
                "Could not upgrade weak reference to RuleNode: {:?} - {:?}",
                self, self.me
            );
        }
        inheritance
    }

    #[tracing::instrument(skip(self))]
    pub fn details(&self) -> Option<String> {
        if let Some(kind) = &self.kind {
            kind.details()
        } else {
            None
        }
    }

    #[tracing::instrument(skip(self))]
    pub fn description(&self) -> Option<String> {
        if let Some(kind) = &self.kind {
            kind.description()
        } else {
            None
        }
    }

    #[tracing::instrument(skip(self, trainz_version))]
    pub fn documentation(&self, trainz_version: &f64) -> Option<String> {
        let obsolete = self.obsolete.and_then(|obsolete| {
            if obsolete >= *trainz_version {
                Some(format!("Obsolete since Trainz build {}", obsolete))
            } else {
                None
            }
        });
        let required = self.compulsory.and_then(|compulsory| {
            if compulsory >= *trainz_version {
                Some(format!("Compulsory since Trainz build {}", compulsory))
            } else {
                None
            }
        });
        let minimum_version = self
            .minimum_version
            .map(|minimum_version| format!("Minimum Trainz build {}", minimum_version));

        let mut docs: Vec<Option<String>> = vec![
            obsolete,
            required,
            minimum_version,
            self.description.clone(),
        ];

        if let Some(kind) = &self.kind {
            docs.push(kind.documentation(trainz_version));
        }

        Some(
            docs.iter()
                .filter_map(|doc| doc.as_ref().map(|doc| doc.to_string()))
                .collect::<Vec<String>>()
                .join("\n\n"),
        )
    }
}

pub(crate) const VALUE_KEY: &str = "Value";
pub(crate) const ELEMENT_KEY: &str = "Element";
pub(crate) const STRUCTURE_KEY: &str = "Structure";
pub(crate) const ARRAY_KEY: &str = "Array";

impl RuleNodeKind {
    #[tracing::instrument(skip(root, key, kind, entries, parent))]
    pub fn new(
        root: &mut RulesRoot,
        key: &str,
        kind: &str,
        entries: &Vec<KeyValuePair>,
        parent: &Option<Weak<RuleNode>>,
    ) -> Option<Self> {
        debug!("Creating new RuleNodeKind: {:?} - {:?}", key, kind);
        if kind.eq_ignore_ascii_case(VALUE_KEY) {
            Some(RuleNodeKind::Value(RuleNodeValue::new(key, entries)?))
        } else if kind.eq_ignore_ascii_case(ELEMENT_KEY) {
            Some(RuleNodeKind::Element(RuleNodeElement::new(entries)))
        } else if kind.eq_ignore_ascii_case(STRUCTURE_KEY) {
            Some(RuleNodeKind::Structure(RuleNodeStructure::new(
                root, entries, false, parent,
            )))
        } else if kind.eq_ignore_ascii_case(ARRAY_KEY) {
            Some(RuleNodeKind::Structure(RuleNodeStructure::new(
                root, entries, true, parent,
            )))
        } else {
            warn!("Unknown kind: {:?} - {:?}", key, kind);
            None
        }
    }

    #[tracing::instrument(skip(self, root))]
    pub(crate) fn update_inheritance(&mut self, root: &RulesRoot) {
        trace!("Updating inheritance for: {:?}", self);
        match self {
            RuleNodeKind::Value(_) => {}
            RuleNodeKind::Element(element) => element.update_inheritance(root),
            RuleNodeKind::Structure(structure) => structure.update_inheritance(root),
        }
    }

    #[tracing::instrument(skip(self, validation_path))]
    pub(crate) async fn update_sources(&mut self, validation_path: &Path) -> Result<()> {
        match self {
            RuleNodeKind::Value(value) => value.update_sources(validation_path).await?,
            RuleNodeKind::Element(element) => element.update_sources(validation_path).await?,
            RuleNodeKind::Structure(structure) => structure.update_sources(validation_path).await?,
        }

        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub fn description(&self) -> Option<String> {
        match self {
            RuleNodeKind::Value(value) => value.description(),
            RuleNodeKind::Element(element) => element.description(),
            RuleNodeKind::Structure(structure) => structure.description(),
        }
    }

    #[tracing::instrument(skip(self))]
    pub fn details(&self) -> Option<String> {
        match self {
            RuleNodeKind::Value(value) => value.details(),
            RuleNodeKind::Element(element) => element.details(),
            RuleNodeKind::Structure(structure) => structure.details(),
        }
    }

    #[tracing::instrument(skip(self))]
    pub fn documentation(&self, trainz_version: &f64) -> Option<String> {
        match self {
            RuleNodeKind::Value(value) => value.documentation(),
            RuleNodeKind::Element(element) => element.documentation(trainz_version),
            RuleNodeKind::Structure(structure) => structure.documentation(trainz_version),
        }
    }
}
