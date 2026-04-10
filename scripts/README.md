# Scripts

This directory contains supporting scripts for the trainz-language-server project.

## Available Scripts

### `create-commit.sh`

Helper script for creating conventional commit messages that follow the project's commit conventions.

**Usage:**
```bash
./scripts/create-commit.sh <type> [scope] <description>
```

**Examples:**
```bash
./scripts/create-commit.sh feat parser "add support for async functions"
./scripts/create-commit.sh fix lsp "prevent deadlock in symbol indexing"
./scripts/create-commit.sh docs "update installation instructions"
```

**Types:**
- `feat` - A new feature
- `fix` - A bug fix
- `perf` - Performance improvement
- `docs` - Documentation changes
- `style` - Code style changes
- `refactor` - Code refactoring
- `test` - Test changes
- `ci` - CI/CD changes
- `chore` - Build/dependencies/tooling

### `commit-agent.sh`

Automated commit script that validates code quality before committing. Runs build, tests, and formatting checks.

**Usage:**
```bash
./scripts/commit-agent.sh "Your commit message"
```

**What it does:**
1. Runs `cargo build` to ensure code compiles
2. Runs `cargo test` to ensure tests pass
3. Runs `cargo fmt --check` to ensure code is formatted
4. Runs `cargo clippy -- -D warnings` to check for linting issues
5. Only commits if all checks pass

### `update-cargo-versions.js`

Node.js script used by semantic-release to update Cargo.toml version fields during automated releases.

**Usage:**
```bash
node scripts/update-cargo-versions.js <version>
```

**What it does:**
- Updates the root `Cargo.toml` workspace version
- Updates all crate `Cargo.toml` files with the new version
- Updates workspace dependency version constraints

This script is automatically run during the semantic release process and should not be run manually.

## Development Workflow

1. **For regular commits:** Use `./scripts/create-commit.sh` to generate properly formatted commit messages
2. **For commits requiring validation:** Use `./scripts/commit-agent.sh` to ensure code quality before committing
3. **Automated releases:** The `update-cargo-versions.js` script runs automatically during semantic releases

See [CONTRIBUTING.md](../CONTRIBUTING.md) and [RELEASES.md](../RELEASES.md) for more details on the development and release process.