use crate::find::{HasRange, position_in_range};
use crate::gs::{ClassDef, Identifier, Include, Scope, Type};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tower_lsp_server::ls_types::Position;

/// Represents a GS program AST.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Program {
    /// Includes in the program.
    pub includes: Vec<Include>,
    /// Classes defined in the program.
    pub classes: HashMap<String, ClassDef>,
    /// Scopes in the program.
    pub scopes: Vec<Scope>,
    /// The ID of the root scope.
    pub root_scope_id: usize,
    /// The range of the program.
    pub range: crate::Range,
    /// The source code.
    pub src: String,
}

impl Program {
    /// Gets a scope by its ID.
    ///
    /// # Arguments
    /// * `id` - The ID of the scope.
    ///
    /// # Returns
    /// An Option containing a reference to the Scope if found.
    ///
    /// # Examples
    ///
    /// ```rust
    /// // let scope = program.get_scope(scope_id);
    /// ```
    pub fn get_scope(&self, id: usize) -> Option<&Scope> {
        self.scopes.get(id)
    }

    /// Finds the narrowest scope that contains the given position.
    ///
    /// # Arguments
    /// * `pos` - The position to search for.
    ///
    /// # Returns
    /// An Option containing a reference to the Scope if found.
    ///
    /// # Examples
    ///
    /// ```rust
    /// // let scope = program.find_narrowest_scope(position);
    /// ```
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

    /// Finds a variable declaration in the scope that contains the given position.
    ///
    /// # Arguments
    /// * `name` - The name of the variable.
    /// * `pos` - The position to search for.
    ///
    /// # Returns
    /// An Option containing a reference to the variable's type and identifier.
    ///
    /// # Examples
    ///
    /// ```rust
    /// // let var_decl = program.find_variable_declaration("my_var", position);
    /// ```
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
