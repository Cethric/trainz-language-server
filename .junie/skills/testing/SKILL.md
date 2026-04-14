---
name: testing
description: Specialized skill for testing and verifying changes in the trainz-lsp project, focusing on Rust unit and integration testing.
---

# Testing Skill

Use this skill when developing, running, or verifying tests within the `trainz-lsp` workspace.

## Key Principles
- **No Regressions**: Always run `cargo test` after changes to verify project health.
- **High Coverage**: Ensure new business logic, parser rules, and LSP features are thoroughly tested.
- **Reproducibility**: Write tests that are deterministic and avoid external dependencies where possible.

## Guidelines
- **Unit Testing**: Place unit tests in the same file as the code they test within a `#[cfg(test)]` module.
- **Integration Testing**: Use the `tests/` directory at the project root for integration tests involving multiple crates.
- **Parser Testing**: Use `crates/parser` to test GS and AcsText grammar parsing using `pest`.
- **LSP Testing**: Verify LSP server behavior in `crates/lsp` using appropriate mock clients or existing test harnesses.
- **Workspace Verification**: Ensure `cargo check` and `cargo clippy` pass for all crates.

## Examples
### Running all tests
1. Execute `cargo test` from the project root.
2. Verify all crates in the workspace pass.

### Running tests for a specific crate
1. Execute `cargo test -p trainz-lsp-parser` (replace with the correct crate name).

## Checklist
- [ ] `cargo test` passes for the entire workspace.
- [ ] New code has corresponding unit tests.
- [ ] Edge cases and negative scenarios are covered.
- [ ] `cargo clippy` reports no warnings.
- [ ] `cargo fmt` has been run to ensure consistent styling.
