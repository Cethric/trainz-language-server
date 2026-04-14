# trainz-language-server

`trainz-language-server` is a Language Server Protocol (LSP) implementation for the **GS (Game Script)** and **AcsText**
languages used in Trainz, written in Rust.

| development                                                                                                                                                                                                                                                   | release                                                                                                                                                                                                                                                    |
|---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| [![Semantic Release](https://github.com/Cethric/trainz-language-server/actions/workflows/release.yml/badge.svg?branch=develop)](https://github.com/Cethric/trainz-language-server/actions/workflows/release.yml)                                              | [![Semantic Release](https://github.com/Cethric/trainz-language-server/actions/workflows/release.yml/badge.svg?branch=main)](https://github.com/Cethric/trainz-language-server/actions/workflows/release.yml)                                              |
| [![LSP Crate Build](https://github.com/Cethric/trainz-language-server/actions/workflows/lsp-build.yml/badge.svg?branch=develop)](https://github.com/Cethric/trainz-language-server/actions/workflows/lsp-build.yml)                                           | [![LSP Crate Build](https://github.com/Cethric/trainz-language-server/actions/workflows/lsp-build.yml/badge.svg?branch=main)](https://github.com/Cethric/trainz-language-server/actions/workflows/lsp-build.yml)                                           |
| [![VSCode Extension Build](https://github.com/Cethric/trainz-language-server/actions/workflows/vscode-extension-build.yml/badge.svg?branch=develop)](https://github.com/Cethric/trainz-language-server/actions/workflows/vscode-extension-build.yml)          | [![VSCode Extension Build](https://github.com/Cethric/trainz-language-server/actions/workflows/vscode-extension-build.yml/badge.svg?branch=main)](https://github.com/Cethric/trainz-language-server/actions/workflows/vscode-extension-build.yml)          |
| [![JetBrains Extension Build](https://github.com/Cethric/trainz-language-server/actions/workflows/jetbrains-extension-build.yml/badge.svg?branch=develop)](https://github.com/Cethric/trainz-language-server/actions/workflows/jetbrains-extension-build.yml) | [![JetBrains Extension Build](https://github.com/Cethric/trainz-language-server/actions/workflows/jetbrains-extension-build.yml/badge.svg?branch=main)](https://github.com/Cethric/trainz-language-server/actions/workflows/jetbrains-extension-build.yml) |

## Features

This language server provides rich editing support for Game Script (`.gs`) and AcsText (`.txt`) files, including:

- **Syntax Highlighting**: Enhanced semantic tokens for GS and AcsText.
- **Diagnostics**: Real-time error reporting and diagnostics.
- **Code Completion**: Context-aware completions for GS.
- **Hover Information**: Detailed information on symbols and variables.
- **Go to Definition**: Navigate through GS source code easily.
- **Formatting**: Automated code formatting for GS.
- **Folding Ranges**: Support for code folding in GS and AcsText.
- **Symbol Indexing**: Quick symbol lookup and navigation.
- **AcsText Validation**: Validation for AcsText files using the `acs_text-validators` crate.

## Tech Stack

- **Language**: Rust (Edition 2024)
- **Runtime**: `tokio` (Async runtime)
- **LSP Framework**: `tower-lsp-server`
- **Parser**: `pest` (Parser generator)
- **Serialization**: `serde` / `serde_json` / `bson`
- **CLI**: `clap`

## Getting Started

### Prerequisites

- **Rust Toolchain**: [Install Rust](https://www.rust-lang.org/tools/install).
- **Cargo**: Usually installed with Rust.

### Building

To build the project in release mode:

```bash
cargo build --release
```

The compiled binary will be located at `target/release/trainz-language-server`.

### Running

To run the LSP server:

```bash
./target/release/language-server
```

#### CLI Options

- `-v, --verbose`: Increase logging verbosity (use multiple `-v` for higher verbosity).
- `-q, --quiet`: Decrease logging verbosity.
- `-p, --validation-path <PATH>`: Path to the directory containing acs_text validators. Can also be set via the
  `TRAINZ_LANGUAGE_SERVER_ACS_TEXT_VALIDATION_PATH` environment variable.
- `-s, --search-paths <PATHS>`: Semicolon-separated list of paths to search for Trainz scripts. Can also be set via the
  `TRAINZ_LANGUAGE_SERVER_SCRIPT_SEARCH_PATHS` environment variable.
- `-h, --help`: Print help.
- `-V, --version`: Print version information.

## Editor Extensions

The project includes extensions for various editors in the `extensions/` directory:

- **Visual Studio Code**: Located in `extensions/vscode`. Requires setting the `trainz-language-server.serverPath` to
  the built `trainz-language-server` binary.
- **IntelliJ IDEA**: Located in `extensions/trainz-idea`.

## Project Structure

The workspace is organized into several crates:

- `crates/ast`: Abstract Syntax Tree definitions.
- `crates/common`: Common types and utilities.
- `crates/parser`: GS and AcsText grammar parsing using `pest`.
- `crates/trainz-language-server`: LSP server implementation.
- `crates/diagnostics`: Diagnostics and error reporting.
- `crates/formatter`: Code formatting logic.
- `crates/formatter-cli`: CLI tool for the formatter.
- `crates/semantic-tokens`: Semantic highlighting support.
- `crates/symboliser`: Symbol indexing and lookup.
- `crates/completions`: Code completion logic.
- `crates/folding`: Folding range support.
- `crates/hover`: Hover information support.
- `crates/definition`: Goto definition support.
- `crates/signature-help`: Signature help support.
- `crates/acs_text-validators`: Validation for AcsText files.
- `crates/util`: General utilities.

## Scripts

The `scripts/` directory contains helper scripts for development and release management:

- **[scripts/create-commit.sh](scripts/create-commit.sh)**: Helper for creating conventional commit messages
- **[scripts/commit-agent.sh](scripts/commit-agent.sh)**: Automated commit script with quality checks
- **[scripts/update-crate-version.js](scripts/update-crate-version.js)**: Update a specific crate's version and its
  workspace dependency version
- **[scripts/update-extension-version.js](scripts/update-extension-version.js)**: Update a specific extension's version
- **[scripts/update-root-versions.js](scripts/update-root-versions.js)**: Update the root project version in
  package.json and Cargo.toml

See [scripts/README.md](scripts/README.md) for detailed usage instructions.

## Development

- **Build**: `cargo build`
- **Test**: `cargo test`
- **Check**: `cargo check`
- **Format**: `cargo fmt`
- **Lint**: `cargo clippy`

Refer to [AGENTS.md](AGENTS.md) for more details on contributing and project guidelines.

## License

This project is part of the `trainz-language-server` effort.
