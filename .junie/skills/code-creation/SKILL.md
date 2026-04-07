---
name: code-creation
description: Specialized skill for generating high-quality Rust code for the gs-lsp project, following Edition 2024 standards and workspace conventions.
---

# Code Creation Skill

Use this skill when generating new Rust code, adding features, or modifying existing logic within the `gs-lsp` workspace.

## Key Principles
- **Workspace Consistency**: Ensure changes are reflected across all relevant crates (e.g., `ast`, `parser`, `symboliser`).
- **Idiomatic Rust**: Use standard Rust idioms, Edition 2024 features, and follow the project's established patterns.
- **Minimalism**: Keep changes focused and avoid unnecessary refactoring during feature development.

## Guidelines
- **Async Execution**: Use `tokio` for async operations where appropriate, aligning with the project's tech stack.
- **Serialization**: Utilize `serde` and `serde_json` for data structures that require serialization.
- **Crate Boundaries**: Respect the workspace structure. Place common types in `crates/common` and utilities in `crates/util`.
- **AST/Parser Integration**: When adding new syntax, update `crates/ast` first, then the grammar in `crates/parser`.

## Examples
### Adding a new AST Node
1. Define the node in `crates/ast/src/lib.rs`.
2. Update the `pest` grammar in `crates/parser/src/gs.pest`.
3. Implement the parsing logic in `crates/parser/src/lib.rs`.

## Checklist
- [ ] Code follows Rust Edition 2024 standards.
- [ ] New features include corresponding updates to `ast` and `parser`.
- [ ] Code is formatted using `cargo fmt`.
- [ ] Public APIs include documentation (///).
