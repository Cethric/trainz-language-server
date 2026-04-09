---
name: documentation
description: Guidelines for generating high-quality Rust documentation for the trainz-lsp project, following standard KDoc (///) practices.
---

# Documentation Skill

Use this skill when documenting the `trainz-lsp` codebase, including public APIs, modules, and internal logic.

## Key Principles
- **Clarity and Precision**: Documentation should clearly describe the purpose, parameters, and return values of public APIs.
- **Consistency**: Follow standard Rust documentation idioms and use KDoc (///) for all public-facing members.
- **Crate-Level Documentation**: Ensure each crate in the workspace has a `lib.rs` with top-level documentation describing its role.

## Guidelines
- **KDoc (///)**: Always use triple-slash comments for public structures, functions, and traits.
- **Internal Comments (//)**: Use for explaining complex internal logic or implementation details.
- **Markdown Support**: Use Markdown features like code blocks and links within KDoc.
- **Workspace-Wide Context**: Document how crates interact (e.g., how `crates/ast` relates to `crates/parser`).

## Examples
### Documenting a Public Function
```rust
/// Parses a Game Script string and returns an AST.
///
/// # Arguments
/// * `input` - The GS source code as a string.
///
/// # Returns
/// A Result containing the Program AST or a ParserError.
pub fn parse_source(input: &str) -> Result<Program, ParserError> { ... }
```

## Checklist
- [ ] Public APIs have clear, descriptive KDoc (///).
- [ ] Module-level documentation exists for each workspace crate.
- [ ] COMPLEX logic includes inline comments for maintainability.
- [ ] Documentation is formatted correctly (e.g., proper use of Markdown).
