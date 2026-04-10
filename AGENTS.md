# AGENTS.md

AGENTS.md is a simple, open format for guiding coding agents. This file provides context and instructions to help AI
coding agents work on the `trainz-language-server` project.

## Project Overview

`trainz-language-server` is a Language Server Protocol (LSP) implementation for the GS (Game Script) language, written
in Rust.

## Tech Stack
- **Language**: Rust (Edition 2024)
- **Libraries**:
    - `tokio`: Async runtime
    - `pest`: Parser generator
    - `tower-lsp-server`: LSP framework
    - `serde`/`serde_json`: Serialization
    - `clap`: CLI argument parsing

## Workspace Structure
The project is organized as a Cargo workspace with the following crates:
- `crates/ast`: Abstract Syntax Tree definitions
- `crates/common`: Common types and utilities
- `crates/parser`: GS and Soup grammar parsing using `pest`
- `crates/lsp`: LSP server implementation
- `crates/diagnostics`: Diagnostics and error reporting
- `crates/formatter`: Code formatting logic
- `crates/formatter-cli`: CLI tool for the formatter
- `crates/semantic-tokens`: Semantic highlighting support
- `crates/symboliser`: Symbol indexing and lookup
- `crates/completions`: Code completion logic
- `crates/folding`: Folding range support
- `crates/hover`: Hover information support
- `crates/definition`: Goto definition support
- `crates/util`: General utilities

## Commands
- **Build**: `cargo build`
- **Test**: `cargo test`
- **Check**: `cargo check`
- **Format**: `cargo fmt`
- **Lint**: `cargo clippy`

## Agent Guidelines
- **Junie**: Focus on maintaining consistency across the workspace. When adding new features, ensure all relevant crates are updated (e.g., adding a new AST node usually requires updating the parser, symboliser, and possibly completions/hover).
- **CommitAgent**: Ensure all changes are valid before committing. This includes verifying that the code builds (`cargo build`), all tests pass (`cargo test`), and the code is well-formatted (`cargo fmt`). Only commit changes when these conditions are met. A helper script `scripts/commit-agent.sh` is provided in the project root to automate this verification and commit process. Usage: `./scripts/commit-agent.sh "Commit message"`.
- **Testing**: Always run `cargo test` after making changes to verify no regressions.
- **Documentation**: Use Rust-standard KDoc (///) for public APIs.
- **Code Style**: Follow standard Rust idioms and the existing formatting in the project.
