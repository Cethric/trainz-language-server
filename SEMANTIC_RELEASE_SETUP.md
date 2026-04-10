# Semantic Release Setup - Implementation Summary

This document summarizes the semantic release process implementation for the trainz-language-server project.

## Files Created/Modified

### Configuration Files

1. **[package.json](package.json)** (ROOT)
   - Added semantic-release configuration
   - Specifies release plugins and behavior
   - Defines branches for stable (main) and pre-release (develop) channels

2. **[.releaserc.json](.releaserc.json)**
   - Detailed semantic-release configuration
   - Event type mappings (feat→minor, fix→patch, perf→patch)
   - Plugin settings for changelog, git, and GitHub integration
   - Asset definitions for release artifacts

3. **[.gitignore](.gitignore)** (UPDATED)
   - Added release-artifacts/ and final-artifacts/ directories
   - Added *.tar.gz and *.zip to ignored patterns

### Documentation Files

1. **[CONTRIBUTING.md](CONTRIBUTING.md)**
   - Conventional Commits specification
   - Commit type and scope guidelines
   - Examples of proper commit messages
   - Development workflow instructions
   - Versioning explanation

2. **[RELEASES.md](RELEASES.md)**
   - Complete guide to automatic releases
   - How the system works end-to-end
   - Branch strategy (main vs develop)
   - Artifact descriptions
   - Troubleshooting guide

### Helper Scripts

1. **[scripts/create-commit.sh](scripts/create-commit.sh)**
   - Interactive script to generate conventional commit messages
   - Usage: `./scripts/create-commit.sh feat parser "add async support"`
   - Helps maintain consistent commit format

### GitHub Actions Workflows

1. **[.github/workflows/lsp-build.yml](.github/workflows/lsp-build.yml)** (EXISTING)
   - Builds LSP crate on push/PR (enhanced with artifact uploads)

2. **[.github/workflows/vscode-extension-build.yml](.github/workflows/vscode-extension-build.yml)** (EXISTING)
   - Builds VSCode extension (enhanced with artifact uploads)

3. **[.github/workflows/jetbrains-extension-build.yml](.github/workflows/jetbrains-extension-build.yml)** (EXISTING)
   - Builds JetBrains plugin (enhanced with artifact uploads)

4. **[.github/workflows/release.yml](.github/workflows/release.yml)** (NEW)
   - Orchestrates semantic release process
   - **Build Stage**:
     - Builds LSP binaries for 4 platforms: Linux x86_64, macOS x86_64, macOS ARM64, Windows x86_64
     - Builds VSCode extension
     - Builds JetBrains plugin
   - **Release Stage**:
     - Downloads all artifacts
     - Runs semantic-release to determine version
     - Creates GitHub release with artifacts
     - Updates version files (package.json, Cargo.toml)
     - Posts release notes

## Release Workflow

### Commit Message Format

Use Conventional Commits: `type(scope): description`

**Example:**
```
feat(parser): add async function support

This allows using async/await syntax in parser definitions.

Closes #123
```

### Version Bumping Rules

| Commit Type | Change | Example |
|------------|--------|---------|
| `feat:` | Minor | 1.0.0 → 1.1.0 |
| `fix:` | Patch | 1.0.0 → 1.0.1 |
| `perf:` | Patch | 1.0.0 → 1.0.1 |
| `BREAKING CHANGE` | Major | 1.0.0 → 2.0.0 |
| `docs:`, `style:`, `refactor:`, `test:`, `ci:`, `chore:` | No release | - |

### Release Artifacts

Each GitHub release includes:

**LSP Binaries:**

- `trainz-language-server-linux-x86_64.tar.gz`
- `trainz-language-server-macos-x86_64.tar.gz`
- `trainz-language-server-macos-aarch64.tar.gz`
- `trainz-language-server-windows-x86_64.zip`

**Extensions:**

- `trainz-language-server-vscode.tar.gz` - VSCode extension build
- `trainz-idea-plugin.tar.gz` - JetBrains plugin distributions

## Integration with Existing CI/CD

The semantic release process integrates with your existing workflows:

1. **Build Workflows** - Run on every push/PR to validate code
2. **Release Workflow** - Runs only on pushed commits to main/develop
3. **Cross-Platform Support** - Builds binaries for all major platforms
4. **Caching** - Uses cargo, pnpm, and Gradle caching for speed

## First Release Setup

To get started:

1. **Ensure npm is available** - The workflows use npm to run semantic-release
2. **Create a GitHub token** - The workflow uses `GITHUB_TOKEN` (automatically available)
3. **Test locally** (optional):
   ```bash
   npm install
   npx semantic-release --dry-run
   ```
4. **Make a conventional commit**:
   ```bash
   git commit -m "feat: initial release"
   ```
5. **Push to main** - First release will be created automatically

## Version File Updates

On release, these files are automatically updated:

- `package.json` - Version field
- `CHANGELOG.md` - Generated release notes
- `crates/*/Cargo.toml` - Version fields (if configured)

## Branch Strategy

- **main** - Stable releases (e.g., v1.0.0, v1.1.0)
- **develop** - Pre-releases (e.g., v1.1.0-next.1, v1.1.0-next.2)

Pre-releases on develop don't trigger new stable version releases and are marked as non-production.

## Next Steps

1. ✅ Review [CONTRIBUTING.md](CONTRIBUTING.md) for commit guidelines
2. ✅ Review [RELEASES.md](RELEASES.md) for release details
3. ✅ Start using the [scripts/create-commit.sh](scripts/create-commit.sh) helper for consistent commits
4. ✅ When ready to release, push to main with conventional commits
5. ✅ GitHub Actions will automatically create releases with artifacts

## Support

- Check [RELEASES.md](RELEASES.md) troubleshooting section
- Review GitHub Actions logs for build failures
- Refer to [Semantic Release docs](https://semantic-release.gitbook.io/)
- Check [Conventional Commits spec](https://www.conventionalcommits.org/)
