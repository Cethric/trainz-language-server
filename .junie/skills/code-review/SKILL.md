---
name: code-review
description: Guidelines for conducting high-quality code reviews in the trainz-lsp workspace, focusing on safety, performance, and workspace consistency.
---

# Code Review Skill

Use this skill when reviewing changes to the `trainz-lsp` codebase.

## Key Principles
- **Workspace Health**: Ensure reviews check for consistency across all workspace crates.
- **Safety First**: Prioritize memory safety and proper error handling in Rust.
- **Maintainability**: Verify that the code follows established project patterns and idioms.

## Guidelines
- **Parser/AST Alignment**: Check if changes in `crates/ast` are correctly reflected in `crates/parser`.
- **LSP Protocol Adherence**: Ensure `crates/lsp` modifications comply with the Language Server Protocol using `tower-lsp-server`.
- **Diagnostics**: Verify that new logic includes appropriate diagnostics and error reporting in `crates/diagnostics`.
- **Semantic Consistency**: Check `crates/symboliser` and `crates/semantic-tokens` when symbols or syntax are modified.

## Examples
### Review Checklist for New Syntax
1. Does the new AST node cover all necessary fields?
2. Is the `pest` grammar in `crates/parser` correctly updated?
3. Are symbols correctly indexed in `crates/symboliser`?
4. Is semantic highlighting updated in `crates/semantic-tokens`?

## Checklist
- [ ] No regressions in existing functionality.
- [ ] Correct use of Rust-standard error handling (`Result`/`Option`).
- [ ] Async code uses `tokio` correctly.
- [ ] No unnecessary dependencies added to `Cargo.toml`.
- [ ] Public API changes include updated KDoc (///).
