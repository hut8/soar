# Release Process

This document describes how to create new releases of SOAR.

## Quick Start

```bash
# Patch release (bug fixes): 0.1.0 → 0.1.1
./release patch

# Minor release (new features): 0.1.0 → 0.2.0
./release minor

# Major release (breaking changes): 0.1.0 → 1.0.0
./release major

# Explicit version
./release 1.2.3

# Test without pushing (dry run)
./release patch --dry-run
```

## What the Script Does

The `./release` script automates the entire release process:

1. **Validates prerequisites**
   - Checks you're on the `main` branch
   - Ensures no uncommitted changes
   - Pulls latest changes from origin

2. **Calculates new version**
   - Reads current version from `Cargo.toml`
   - Applies semantic versioning rules (major.minor.patch)
   - Or accepts explicit version number

3. **Updates version files**
   - `Cargo.toml` - Rust backend version
   - `web/package.json` - Frontend version
   - `web/package-lock.json` - Auto-updated by npm
   - `Cargo.lock` - Auto-updated by cargo

4. **Creates version commit**
   - Commits all version changes
   - Uses message: "chore: bump version to X.Y.Z"

5. **Tags the release**
   - Creates annotated git tag: `vX.Y.Z`
   - Tag message: "Release vX.Y.Z"

6. **Pushes to GitHub**
   - Pushes the commit to `origin/main`
   - Pushes the tag to `origin`
   - **This triggers the GitHub Actions release workflow**

## What Happens After Push

Once the tag is pushed, the [Release workflow](.github/workflows/release.yml) automatically:

1. **Builds release binaries** for multiple platforms:
   - Linux x64 (glibc)
   - Linux x64 (musl)
   - macOS x64
   - macOS ARM64
   - Windows x64

2. **Uploads debug symbols** to Sentry (if configured)

3. **Builds and pushes Docker image** to GitHub Container Registry:
   - `ghcr.io/OWNER/soar:latest`
   - `ghcr.io/OWNER/soar:X.Y.Z`

4. **Creates GitHub Release** with:
   - Release notes
   - Binary downloads
   - Docker pull instructions

## Semantic Versioning

SOAR follows [Semantic Versioning](https://semver.org/):

- **MAJOR** version (X.0.0): Incompatible API changes or breaking changes
- **MINOR** version (0.X.0): New features, backwards compatible
- **PATCH** version (0.0.X): Bug fixes, backwards compatible

### When to Bump Each Version

**Patch (0.0.X)**
- Bug fixes
- Security patches
- Documentation updates
- Performance improvements (no API changes)

**Minor (0.X.0)**
- New features
- New API endpoints
- New configuration options
- Deprecations (but not removals)
- Database migrations (backwards compatible)

**Major (X.0.0)**
- Breaking API changes
- Removal of deprecated features
- Database schema changes requiring manual migration
- Configuration file format changes
- Minimum dependency version bumps that affect users

## Manual Release (Without Script)

If you need to create a release manually:

```bash
# 1. Update versions
sed -i 's/version = "0.1.0"/version = "0.2.0"/' Cargo.toml
cd web && npm version 0.2.0 --no-git-tag-version && cd ..

# 2. Update lock files
cargo check

# 3. Commit and tag
git add Cargo.toml Cargo.lock web/package.json web/package-lock.json
git commit -m "chore: bump version to 0.2.0"
git tag -a v0.2.0 -m "Release v0.2.0"

# 4. Push
git push origin main
git push origin v0.2.0
```

## Troubleshooting

### "Uncommitted changes detected"

Commit or stash your changes before releasing:
```bash
git add .
git commit -m "your changes"
# OR
git stash
```

### "Must be on main branch"

Switch to main and pull latest:
```bash
git checkout main
git pull
```

### Release workflow failed

Check the GitHub Actions logs:
1. Go to your repository on GitHub
2. Click "Actions" tab
3. Find the failed "Release" workflow
4. Click on it to see error details

Common failures:
- **Sentry upload failed**: Check `SENTRY_AUTH_TOKEN` secret is set
- **Docker push failed**: Check `GITHUB_TOKEN` permissions
- **Build failed**: Check if code compiles locally first

### Need to delete a bad release

```bash
# Delete local tag
git tag -d v1.0.0

# Delete remote tag
git push origin --delete v1.0.0

# Delete GitHub release (in browser)
# Go to Releases → Click the release → Delete
```

### Accidentally pushed wrong version

If you haven't deleted the tag yet:
1. Delete the tag (see above)
2. Reset your local branch: `git reset --hard HEAD~1`
3. Fix the version and re-run `./release`

## Pre-release Checklist

Before creating a release, ensure:

- [ ] All tests pass (`cargo test`, `npm test`)
- [ ] CI is green on main branch
- [ ] `CHANGELOG.md` is updated (if you maintain one)
- [ ] Breaking changes are documented
- [ ] Database migrations are tested
- [ ] No uncommitted changes
- [ ] You're on the `main` branch
- [ ] You've pulled latest changes

## Post-release Checklist

After creating a release:

- [ ] Verify GitHub Release was created
- [ ] Check Docker images are available on ghcr.io
- [ ] Test installation from release binaries
- [ ] Update deployment (if manual)
- [ ] Announce release (if applicable)
- [ ] Close related GitHub issues/milestones

## See Also

- [GitHub Actions Release Workflow](.github/workflows/release.yml)
- [Sentry Debug Symbols](sentry-debug-symbols.md)
- [Semantic Versioning](https://semver.org/)
