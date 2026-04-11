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

### `update-crate-version.js`

Node.js script to update the version of a specific crate and its corresponding workspace dependency.

**Usage:**

```bash
node scripts/update-crate-version.js <crate_name> <new_version>
```

**What it does:**

- Updates the version in `crates/<crate_name>/Cargo.toml`.
- Updates the version of `trainz-<crate_name>` in the root `Cargo.toml`.

### `update-extension-version.js`

Node.js script to update the version of a specific editor extension.

**Usage:**
```bash
node scripts/update-extension-version.js <extension_name> <new_version>
```

**What it does:**

- Updates `version` in `extensions/vscode/package.json` (if `vscode`)
- Updates `version` in `extensions/trainz-idea/build.gradle.kts` (if `trainz-idea`)

### `update-root-versions.js`

Node.js script to update the root package and workspace versions.

**Usage:**

```bash
node scripts/update-root-versions.js <new_version>
```

**What it does:**

- Updates `version` in the root `package.json`.
- Updates `version` in the root `Cargo.toml` under `[workspace.package]`.

## Development Workflow

1. **For regular commits:** Use `./scripts/create-commit.sh` to generate properly formatted commit messages
2. **For commits requiring validation:** Use `./scripts/commit-agent.sh` to ensure code quality before committing
3. **Version updates:** The `update-crate-version.js` script is used to update crate versions,
   `update-extension-version.js` for extension versions, and `update-root-versions.js` for the root project version.

See [CONTRIBUTING.md](../CONTRIBUTING.md) and [RELEASES.md](../RELEASES.md) for more details on the development and release process.