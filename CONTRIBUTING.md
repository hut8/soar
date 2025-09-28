# Contributing to SOAR

Thank you for your interest in contributing to SOAR! This guide will help you get started with development.

## Table of Contents

- [Prerequisites](#prerequisites)
- [Development Setup](#development-setup)
- [Project Structure](#project-structure)
- [Development Workflow](#development-workflow)
- [Code Style and Standards](#code-style-and-standards)
- [Testing](#testing)
- [Database Development](#database-development)
- [Troubleshooting](#troubleshooting)

## Prerequisites

Before you start, ensure you have the following installed:

### Required Tools

1. **Rust** (latest stable)
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   source ~/.cargo/env
   ```

2. **Node.js** (version 20 or higher)
   - Download from [nodejs.org](https://nodejs.org/)
   - Or use a version manager like [nvm](https://github.com/nvm-sh/nvm)

3. **PostgreSQL** (version 15 or higher) with PostGIS
   ```bash
   # Ubuntu/Debian
   sudo apt-get install postgresql postgresql-contrib postgis

   # macOS (with Homebrew)
   brew install postgresql postgis

   # Start PostgreSQL service
   sudo systemctl start postgresql  # Linux
   brew services start postgresql   # macOS
   ```

4. **Git** (for version control)

### Optional but Recommended

- **Docker** (for isolated database testing)
- **pgAdmin** or **psql** (for database management)

## Development Setup

### 1. Clone the Repository

```bash
git clone <repository-url>
cd soar
```

### 2. Install Development Tools

Run our automated setup script:

```bash
# This installs Diesel CLI, cargo-audit, cargo-outdated, and web dependencies
./scripts/install-dev-tools.sh
```

### 3. Set Up Pre-commit Hooks

```bash
# This installs pre-commit and configures hooks that match our CI pipeline
./scripts/setup-precommit.sh
```

### 4. Configure Environment

```bash
# Copy the example environment file
cp .env.example .env

# Edit .env with your settings
# Key variables to configure:
# - DATABASE_URL: Your PostgreSQL connection string
# - NATS_URL: NATS server URL (default: nats://localhost:4222)
# - GOOGLE_MAPS_API_KEY: For geocoding features
```

### 5. Set Up Databases

#### Development Database
```bash
# Create the main development database
createdb soar_dev
psql -d soar_dev -c "CREATE EXTENSION IF NOT EXISTS postgis;"
psql -d soar_dev -c "CREATE EXTENSION IF NOT EXISTS pg_trgm;"

# Set DATABASE_URL for development
export DATABASE_URL="postgres://username:password@localhost:5432/soar_dev"

# Run migrations
diesel migration run
```

#### Test Database
```bash
# Create the test database
createdb soar_test
psql -d soar_test -c "CREATE EXTENSION IF NOT EXISTS postgis;"
psql -d soar_test -c "CREATE EXTENSION IF NOT EXISTS pg_trgm;"

# Test database URL (used by tests and pre-commit hooks)
export DATABASE_URL="postgres://username:password@localhost:5432/soar_test"
diesel migration run
```

### 6. Install NATS Server

```bash
# Download and install NATS server
wget https://github.com/nats-io/nats-server/releases/download/v2.11.8/nats-server-v2.11.8-amd64.deb
sudo dpkg -i nats-server-*.deb
rm nats-server-*.deb

# Start NATS server
nats-server &
```

### 7. Verify Setup

```bash
# Test Rust build
cargo build

# Test web build
cd web && npm run build && cd ..

# Run all pre-commit checks
pre-commit run --all-files

# Run tests
cargo test
cd web && npm test && cd ..
```

## Project Structure

```
soar/
├── src/                    # Rust source code
│   ├── actions/           # HTTP route handlers
│   ├── aprs_client/       # APRS-IS client implementation
│   ├── web.rs            # Web server setup
│   └── main.rs           # Application entry point
├── web/                   # SvelteKit frontend
│   ├── src/
│   │   ├── routes/       # SvelteKit pages
│   │   ├── lib/          # Shared components and utilities
│   │   └── app.html      # HTML template
│   └── package.json
├── migrations/            # Database migrations (Diesel)
├── scripts/              # Development and deployment scripts
├── .github/workflows/    # CI/CD configuration
├── .pre-commit-config.yaml # Pre-commit hook configuration
└── README.md
```

## Development Workflow

### 1. Before Starting Work

```bash
# Create a new branch for your feature/fix
git checkout -b feature/your-feature-name

# Make sure your environment is up to date
git pull origin main
./scripts/install-dev-tools.sh  # If dependencies changed
```

### 2. Making Changes

- Write your code following the established patterns
- Add tests for new functionality
- Update documentation as needed
- Run pre-commit hooks: `pre-commit run --all-files`

### 3. Before Committing

```bash
# Format code
cargo fmt
cd web && npm run format && cd ..

# Run all checks locally (same as CI)
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cd web && npm run lint && npm run check && npm test && cd ..

# Or let pre-commit handle it
pre-commit run --all-files
```

### 4. Committing Changes

```bash
# Stage your changes
git add .

# Commit (pre-commit hooks will run automatically)
git commit -m "feat: add your feature description"

# Push to your branch
git push origin feature/your-feature-name
```

## Code Style and Standards

### Rust Code Style

- Use `cargo fmt` for formatting (enforced by CI)
- Follow Rust naming conventions
- Use `clippy` for linting (enforced by CI)
- Write documentation for public APIs
- Use `anyhow::Result` for error handling
- Prefer `tracing` over `println!` for logging

### Frontend Code Style

- Use Prettier for formatting (enforced by CI)
- Follow TypeScript best practices
- Use ESLint configuration (enforced by CI)
- Write type-safe code with proper TypeScript types
- Use Svelte 5 syntax and patterns

### Database Conventions

- Use descriptive table and column names
- Include proper indexes for queries
- Write both `up.sql` and `down.sql` for migrations
- Use UUID primary keys for new tables
- Include proper foreign key constraints

### Git Commit Messages

Follow conventional commit format:
```
type(scope): description

feat: add new feature
fix: correct bug
docs: update documentation
style: formatting changes
refactor: code restructuring
test: add or update tests
chore: maintenance tasks
```

## Testing

### Rust Tests

```bash
# Run all Rust tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_name

# Run tests with coverage (requires cargo-tarpaulin)
cargo tarpaulin --out html
```

### Frontend Tests

```bash
cd web

# Run E2E tests with Playwright
npm test

# Run tests in headed mode (see browser)
npm run test:headed

# Run specific test file
npx playwright test tests/example.spec.ts
```

### Database Tests

- Tests automatically use the test database
- Database is reset between test runs
- Use transactions in tests to avoid side effects

## Database Development

### Creating Migrations

```bash
# Create a new migration
diesel migration generate migration_name

# This creates two files:
# migrations/yyyy-mm-dd-hhmmss_migration_name/up.sql
# migrations/yyyy-mm-dd-hhmmss_migration_name/down.sql
```

### Migration Best Practices

1. **Always test both up and down migrations**:
   ```bash
   diesel migration run
   diesel migration revert
   diesel migration run
   ```

2. **Use transactions for complex migrations**:
   ```sql
   -- up.sql
   BEGIN;
   -- Your migration commands here
   COMMIT;
   ```

3. **Add indexes for performance**:
   ```sql
   CREATE INDEX CONCURRENTLY idx_table_column ON table_name (column_name);
   ```

### Working with PostGIS

- Use proper spatial indexes: `CREATE INDEX ON table_name USING GIST (geom_column);`
- Use appropriate SRID (4326 for WGS84 lat/lng)
- Test spatial queries thoroughly

## Troubleshooting

### Common Issues

1. **Pre-commit hooks failing**:
   ```bash
   # Fix formatting issues
   cargo fmt
   cd web && npm run format && cd ..

   # Update hooks
   pre-commit autoupdate
   ```

2. **Database connection errors**:
   ```bash
   # Check PostgreSQL is running
   sudo systemctl status postgresql

   # Check database exists
   psql -l

   # Verify DATABASE_URL in .env
   echo $DATABASE_URL
   ```

3. **Rust compilation errors**:
   ```bash
   # Clean build cache
   cargo clean

   # Update dependencies
   cargo update

   # Check Rust version
   rustc --version
   ```

4. **Web build errors**:
   ```bash
   cd web

   # Clear node_modules and reinstall
   rm -rf node_modules package-lock.json
   npm install

   # Clear build cache
   rm -rf .svelte-kit build
   ```

5. **NATS connection issues**:
   ```bash
   # Check NATS is running
   ps aux | grep nats-server

   # Start NATS server
   nats-server &

   # Check NATS_URL in .env
   ```

### Getting Help

- Check existing [GitHub Issues](../../issues)
- Review the [GitHub Discussions](../../discussions)
- Look at CI logs for similar failures
- Ask questions in the project chat/forum

### Development Tips

1. **Use cargo-watch for auto-rebuilding**:
   ```bash
   cargo install cargo-watch
   cargo watch -x run
   ```

2. **Use browser dev tools for frontend debugging**

3. **Enable detailed logging**:
   ```bash
   export RUST_LOG=debug
   cargo run
   ```

4. **Use database GUI tools**:
   - pgAdmin for PostgreSQL management
   - PostGIS extensions for spatial data

---

Thank you for contributing to SOAR! Your help makes this project better for everyone.