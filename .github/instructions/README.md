# GitHub Copilot Path-Specific Instructions

This directory contains path-specific instruction files for GitHub Copilot. These files provide focused guidance for different types of code in the repository.

## How It Works

Each `.instructions.md` file contains:
- YAML frontmatter specifying which files it applies to (using glob patterns)
- Markdown content with coding standards, patterns, and examples

GitHub Copilot automatically reads these files and uses them as context when generating code for matching file paths.

## Current Instructions

### rust-backend.instructions.md
- **Applies to**: `src/**/*.rs`
- **Purpose**: Rust backend development standards
- **Key topics**:
  - Diesel ORM usage
  - Error handling with `anyhow`
  - Logging with `tracing`
  - Metrics conventions
  - Repository patterns
  - Axum API handlers

### frontend-svelte.instructions.md
- **Applies to**: `web/src/**/*.{ts,svelte}`
- **Purpose**: Frontend development with SvelteKit 5 and TypeScript
- **Key topics**:
  - Svelte 5 syntax (critical: `onclick` not `on:click`)
  - Component structure with runes
  - Icon usage with Lucide
  - Static site generation (no SSR)
  - API communication patterns

### playwright-tests.instructions.md
- **Applies to**: `web/e2e/**/*.spec.ts`
- **Purpose**: E2E testing standards with Playwright
- **Key topics**:
  - Test structure and isolation
  - Stable locator strategies
  - Auto-wait patterns
  - Test data management
  - Page Object Model

## Benefits

1. **Focused Context**: Copilot gets specific guidance based on what file you're editing
2. **Reduced Conflicts**: Path-specific rules override general rules when needed
3. **Better Organization**: Keep instructions organized by code type
4. **Maintainability**: Easier to update rules for specific file types

## Adding New Instructions

To add instructions for a new file type:

1. Create a new `.instructions.md` file in this directory
2. Add YAML frontmatter with `applyTo` glob pattern:
   ```yaml
   ---
   applyTo: "path/to/files/**/*.ext"
   excludeAgent: "code-review"  # Optional: exclude from specific agents
   ---
   ```
3. Write your instructions in Markdown below the frontmatter
4. Commit and push the file

## Examples

### SQL Migration Instructions
```yaml
---
applyTo: "migrations/**/*.sql"
---

# Database Migration Standards

- Always use transactions
- Never use `CREATE INDEX CONCURRENTLY` (not supported in transactions)
- Include rollback instructions in comments
```

### API Documentation Instructions
```yaml
---
applyTo: "docs/api/**/*.md"
---

# API Documentation Standards

- Use OpenAPI 3.0 format
- Include examples for all endpoints
- Document error responses
```

## Related Files

- `../copilot-instructions.md` - Repository-wide instructions (applies everywhere)
- `../../CLAUDE.md` - Comprehensive AI assistant guide
- `../copilot-setup-steps.yml` - Development environment setup

## Learn More

- [GitHub Copilot Custom Instructions](https://docs.github.com/en/copilot/customizing-copilot/adding-repository-custom-instructions-for-github-copilot)
- [Path-specific instructions blog post](https://github.blog/changelog/2025-07-23-github-copilot-coding-agent-now-supports-instructions-md-custom-instructions/)
