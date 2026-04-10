# Release Management

## Automatic Releases

This project uses [semantic-release](https://semantic-release.gitbook.io/) to automatically manage versioning and create GitHub releases.

### How It Works

1. **Commits are analyzed** - When commits are pushed to `main` or `develop`, semantic-release analyzes commit messages
2. **Version is determined** - Based on [Conventional Commits](https://www.conventionalcommits.org/), a new version is calculated
3. **Build artifacts are created** - LSP binaries for all platforms and extensions are built and packaged
4. **GitHub release is created** - A new release is created with all artifacts attached for download
5. **Version files are updated** - `package.json` and `Cargo.toml` files are automatically updated

### Commit Message Convention

Use [Conventional Commits](https://www.conventionalcommits.org/) format:

```
<type>[optional scope]: <description>
```

**Types that trigger releases:**
- `feat:` → Minor version bump (e.g., 1.0.0 → 1.1.0)
- `fix:` → Patch version bump (e.g., 1.0.0 → 1.0.1)
- `perf:` → Patch version bump (e.g., 1.0.0 → 1.0.1)
- `BREAKING CHANGE:` → Major version bump (e.g., 1.0.0 → 2.0.0)

**Other types do NOT trigger releases:**
- `docs:`, `style:`, `refactor:`, `test:`, `ci:`, `chore:`

### Helper Script

We provide a helper script for creating conventional commit messages:

```bash
./scripts/create-commit.sh feat parser "add support for async functions"
```

This will output the formatted commit message you should use.

### Example Workflow

1. Make your changes
2. Stage files: `git add .`
3. Create a commit with conventional format:
   ```bash
   git commit -m "feat(parser): add support for async functions

   This adds the ability to destructure objects and arrays in function
   parameter lists, making the syntax more consistent with modern JavaScript.
   
   Closes #123"
   ```
4. Push to `main` or `develop`:
   ```bash
   git push origin main
   ```
5. GitHub Actions automatically:
   - Validates the build
   - Determines the new version
   - Builds all binaries and extensions
   - Creates a GitHub release with all artifacts

### Release Artifacts

Each release includes:

- **LSP Binaries** - Compiled `trainz-language-server` and `trainz-fmt` for:
  - Linux x86_64
  - macOS x86_64
  - macOS ARM64 (Apple Silicon)
  - Windows x86_64

- **VSCode Extension** - `trainz-language-server-vscode.tar.gz` containing the compiled extension

- **JetBrains Plugin** - `trainz-idea-plugin.tar.gz` containing the plugin distributions

### Branches

- **main** - Stable releases (version numbers: 1.0.0, 1.1.0, etc.)
- **develop** - Pre-releases (version numbers: 1.0.0-next.1, 1.0.0-next.2, etc.)

### Configuration

Release configuration is in [`.releaserc.json`](.releaserc.json). Key settings:

- **Branches**: Which branches trigger releases
- **Plugins**: Tools used for analyzing commits, generating release notes, and publishing
- **NPM Scripts**: Commands run during the release process

View the file to see detailed configuration for version determination and release notes generation.

### Troubleshooting

**No release created even with a `feat:` commit**
- Check that the commit message exactly follows Conventional Commits format
- Ensure changes were pushed to `main` or `develop`
- Check GitHub Actions workflow logs

**Wrong version bumped**
- Review the commit message format
- Check `.releaserc.json` for type-to-version mappings

**Artifacts missing from release**
- Check GitHub Actions build logs
- Verify artifact paths in `.github/workflows/release.yml`
- Ensure all jobs completed successfully

### Manual Release (if needed)

To manually trigger a release without changing code:

1. Push a commit with the desired update (e.g., update CHANGELOG.md)
2. GitHub Actions will automatically determine if a release is needed
3. Or create an empty commit: `git commit --allow-empty -m "chore(release): trigger release"` (only works if allowed by configuration)

### See Also

- [Contributing Guide](./CONTRIBUTING.md)
- [Semantic Release Documentation](https://semantic-release.gitbook.io/)
- [Conventional Commits](https://www.conventionalcommits.org/)
