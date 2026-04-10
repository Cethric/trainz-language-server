# Contributing to trainz-language-server

Thank you for your interest in contributing to trainz-language-server! This document provides guidelines and
instructions for contributing.

## Commit Message Convention

We follow the [Conventional Commits](https://www.conventionalcommits.org/) specification. This ensures that commit messages are clear, consistent, and allows for automated version management.

### Format

```
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

### Type

Must be one of the following:

- **feat**: A new feature
- **fix**: A bug fix
- **perf**: A code change that improves performance
- **docs**: Documentation only changes
- **style**: Changes that don't affect code meaning (formatting, whitespace, etc.)
- **refactor**: A code change that neither fixes a bug nor adds a feature
- **test**: Adding or updating tests
- **ci**: Changes to CI/CD configuration
- **chore**: Build process, dependencies, or tooling changes

### Scope

Optional. Specifies the area of the codebase affected:
- `parser`: Changes to the parser crate
- `lsp`: Changes to the LSP server
- `ast`: Changes to the AST crate
- `diagnostics`: Changes to diagnostics
- `formatter`: Changes to code formatting
- `completions`: Changes to code completion
- `hover`: Changes to hover support
- `definition`: Changes to go-to-definition
- `vscode`: Changes to the VSCode extension
- `jetbrains`: Changes to the JetBrains plugin

### Description

A short summary of the change (imperative mood, no capital letter, no period):
- ✅ "add support for async functions"
- ✅ "fix parser handling of empty arrays"
- ❌ "Added support for async functions"
- ❌ "Fixed parser handling"

### Body

Optional. Provide additional context:
- Why the change was made
- What was changed and why
- Any trade-offs or side effects

### Footer

Optional. Reference issues and breaking changes:

```
Closes #123
Fixes #456
```

For breaking changes:

```
BREAKING CHANGE: description of breaking change
```

### Examples

```
feat(parser): add support for destructuring in function parameters

This adds the ability to destructure objects and arrays in function
parameter lists, making the syntax more consistent with modern JavaScript.

Closes #123
```

```
fix(lsp): prevent deadlock in symbol indexing

The semaphore order has been reversed to prevent potential deadlocks
when multiple requests are processed concurrently.

Fixes #456
```

```
perf(formatter): optimize line-wrapping algorithm

Implements a more efficient algorithm for determining optimal line breaks,
reducing formatting time by approximately 30%.
```

```
docs: update installation instructions for macOS

BREAKING CHANGE: Homebrew formula has been moved to a new tap
```

## Development Workflow

1. **Fork the repository** and create a feature branch
2. **Make your changes** and test them thoroughly
3. **Write a clear commit message** following the conventions above
4. **Push to your branch** and create a Pull Request
5. **Wait for reviews** and address feedback

## Versioning

We use [Semantic Versioning](https://semver.org/):

- **Major** (X.0.0): Breaking changes
- **Minor** (0.X.0): New features (backward-compatible)
- **Patch** (0.0.X): Bug fixes

Versions are automatically determined based on commit messages:

- `BREAKING CHANGE:` → Major version bump
- `feat:` → Minor version bump
- `fix:` or `perf:` → Patch version bump

## Release Process

Releases are automatically created when commits are merged to:

- **main**: Creates a stable release
- **develop**: Creates a pre-release (next channel)

All artifacts including LSP binaries and extensions are automatically packaged and attached to the GitHub release.

## Code Quality

Before committing, ensure:

```bash
# Format code
cargo fmt

# Run clippy lints
cargo clippy -- -D warnings

# Run tests
cargo test

# Build successfully
cargo build
```

## Getting Help

- Open an issue for bug reports
- Discuss major changes before starting work
- Ask questions in issues before submitting PRs
