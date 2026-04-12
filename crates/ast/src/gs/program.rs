use crate::find::{HasRange, position_in_range};
use crate::gs::{ClassDef, Identifier, Include, Scope, Type};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tower_lsp_server::ls_types::Position;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Program {
    pub includes: Vec<Include>,
    pub classes: HashMap<String, ClassDef>,
    pub scopes: Vec<Scope>,
    pub root_scope_id: usize,
    pub range: crate::Range,
    pub src: String,
}

impl Program {
    pub fn get_scope(&self, id: usize) -> Option<&Scope> {
        self.scopes.get(id)
    }

    pub fn find_narrowest_scope(&self, pos: Position) -> Option<&Scope> {
        self.find_narrowest_scope_recursive(self.root_scope_id, pos)
    }

    fn find_narrowest_scope_recursive(&self, scope_id: usize, pos: Position) -> Option<&Scope> {
        let scope = self.get_scope(scope_id)?;
        if !position_in_range(pos, scope.range) {
            return None;
        }

        for &child_id in &scope.children {
            if let Some(child_scope) = self.find_narrowest_scope_recursive(child_id, pos) {
                return Some(child_scope);
            }
        }

        Some(scope)
    }

    pub fn find_variable_declaration(
        &self,
        name: &str,
        pos: Position,
    ) -> Option<(&Type, &Identifier)> {
        let mut scope = self.find_narrowest_scope(pos)?;
        loop {
            for (ty, id) in &scope.variables {
                if id.name == name {
                    // Check if declaration is before the usage position
                    if id.range.start <= pos {
                        return Some((ty, id));
                    }
                }
            }

            if let Some(parent_id) = scope.parent {
                scope = self.get_scope(parent_id)?;
            } else {
                break;
            }
        }
        None
    }
}

impl HasRange for Program {
    fn range(&self) -> crate::Range {
        self.range
    }
}
