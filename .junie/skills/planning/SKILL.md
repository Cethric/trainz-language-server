---
name: planning
description: Strategic planning skill for mapping out feature implementation and workspace-wide changes in the gs-lsp project.
---

# Planning Skill

Use this skill when preparing for a task, designing features, or mapping out changes across multiple crates.

## Key Principles
- **Holistic View**: Consider the impact of changes across the entire `gs-lsp` workspace.
- **Incremental Progress**: Break down large tasks into smaller, manageable steps.
- **Technical Alignment**: Ensure plans align with the project's tech stack (Rust 2024, `tokio`, `pest`).

## Guidelines
- **Crate Analysis**: Identify which crates need modification (e.g., `ast` -> `parser` -> `symboliser` -> `lsp`).
- **Test Strategy**: Plan for verification at each stage, from unit tests to workspace-level integration tests.
- **Dependencies**: Evaluate if any new dependencies are truly necessary and follow established crate patterns.
- **LSP Protocol**: Ensure feature implementation aligns with the `tower-lsp-server` framework and LSP specifications.

## Examples
### Planning a New Feature (e.g., "Go to Definition")
1. Update `crates/ast` with necessary location metadata.
2. Modify `crates/parser` to capture locations during parsing.
3. Implement indexing logic in `crates/symboliser`.
4. Add the LSP handler in `crates/lsp`.
5. Verify using existing integration tests and mock GS files.

## Checklist
- [ ] Task is broken down into incremental steps.
- [ ] All affected workspace crates are identified.
- [ ] Test plan is included for each major stage.
- [ ] Potential risks or breaking changes are documented.
- [ ] Plan aligns with existing architectural patterns.
