# Scripts

This directory contains database management, deployment, and utility scripts.

## Deployment Script

The `deploy` script allows you to deploy SOAR either locally (on the same server) or remotely (to a production server), mimicking the GitHub Actions deployment workflow.

### Usage

```bash
# Deploy current branch (will prompt for local or remote)
./scripts/deploy

# Deploy a specific branch
./scripts/deploy my-branch

# Local deployment (on the same server)
LOCAL_DEPLOY=1 ./scripts/deploy
DEPLOY_SERVER=local ./scripts/deploy

# Remote deployment
DEPLOY_SERVER=prod.example.com ./scripts/deploy
```

### Environment Variables

The deployment script supports the following optional environment variables:

```bash
# Sentry integration (optional)
SENTRY_AUTH_TOKEN=your-token      # Sentry authentication token
SENTRY_ORG=your-org               # Sentry organization slug
SENTRY_PROJECT=your-project       # Sentry project slug

# Deployment mode
LOCAL_DEPLOY=1                    # Force local deployment (on same server)
DEPLOY_SERVER=local               # 'local'/'localhost' for local, or hostname for remote

# SSH configuration (for remote deployment only)
SSH_PRIVATE_KEY_PATH=~/.ssh/id_rsa  # Path to SSH private key (default: ~/.ssh/id_rsa)

# Deployment options
SKIP_SENTRY=1                     # Skip Sentry debug symbols and release creation
SKIP_TESTS=1                      # Skip running tests (not recommended)
```

### Prerequisites

**For all deployments:**
1. Node.js and Rust toolchain for building
2. sentry-cli installed (optional, for Sentry integration)
3. The deployment script (`/usr/local/bin/soar-deploy`) installed on the target server

**For local deployment:**
- Must be run on the deployment server itself
- Requires sudo access to run `/usr/local/bin/soar-deploy`

**For remote deployment:**
- SSH access to the deployment server as the `soar` user
- SSH key authentication configured

### Deployment Process

The script performs the following steps:

1. Checks out the specified branch (or uses current branch)
2. Runs tests (cargo fmt, clippy, cargo test, npm lint, npm check)
3. Builds the web frontend (`npm run build`)
4. Builds the Rust release binary (`cargo build --release`)
5. Uploads debug symbols to Sentry (if configured)
6. Creates a Sentry release (if configured)
7. Prepares deployment package including:
   - `soar` binary
   - `infrastructure/soar-deploy` script
   - `*.service` and `*.timer` systemd files
   - Prometheus job configurations
   - Grafana provisioning and dashboards
   - Backup scripts
   - VERSION file with commit SHA
8. **For local deployment**: Copies package to `/tmp/soar/deploy/` and invokes `/usr/local/bin/soar-deploy` directly
   **For remote deployment**: Uploads package to server via SSH and executes deployment script remotely
9. The `soar-deploy` script stops services, runs migrations, installs files, and restarts services

### Examples

```bash
# Local deployment on the server itself
LOCAL_DEPLOY=1 ./scripts/deploy main

# Local deployment with Sentry integration
SENTRY_AUTH_TOKEN=xxx SENTRY_ORG=my-org SENTRY_PROJECT=soar \
LOCAL_DEPLOY=1 ./scripts/deploy

# Remote deployment with all features
SENTRY_AUTH_TOKEN=xxx SENTRY_ORG=my-org SENTRY_PROJECT=soar \
DEPLOY_SERVER=prod.example.com \
./scripts/deploy main

# Remote deployment without Sentry, skipping tests (for testing)
SKIP_SENTRY=1 SKIP_TESTS=1 DEPLOY_SERVER=staging.example.com \
./scripts/deploy feat/my-feature

# Interactive mode (will prompt for local vs remote)
./scripts/deploy
# When prompted, enter "local" for local deployment or hostname for remote
```

### Safety Features

- Confirms uncommitted changes before proceeding
- Shows deployment package contents
- Tests SSH connection before uploading
- Requires manual confirmation before executing deployment
- Returns to original branch after deployment (if branch was switched)

---

## Database Reset Script

**⚠️ WARNING: These scripts will permanently DROP and RECREATE the entire database!**

### Files

- `reset-db.py` - Main Python script that drops and recreates the database
- `reset-db.sh` - Shell wrapper for easier execution

### Prerequisites

- Python 3 with `psycopg` (psycopg3) library installed
- Database connection access
- Environment variables for database connection (optional)

### Environment Variables

The script will use these environment variables if available:

```bash
DB_HOST=localhost      # Database host (default: localhost)
DB_PORT=5432          # Database port (default: 5432)
DB_USER=postgres      # Database user (default: postgres)
DB_PASSWORD=secret    # Database password (optional)
```

### Usage

#### Python Script

```bash
# Dry run (safe - shows what would be done)
./scripts/reset-db.py dev --dry-run
./scripts/reset-db.py production --dry-run

# Actual reset (DANGEROUS!)
./scripts/reset-db.py dev
./scripts/reset-db.py production
```

#### Shell Wrapper

```bash
# Dry run
./scripts/reset-db.sh dev --dry-run
./scripts/reset-db.sh production --dry-run

# Actual reset
./scripts/reset-db.sh dev
./scripts/reset-db.sh production
```

### Safety Features

1. **Multiple confirmations** - requires typing specific phrases
2. **Extra confirmation for production** - additional safety step
3. **Dry run mode** - see what would be done without making changes
4. **Force disconnect clients** - terminates all connections before dropping
5. **Database existence check** - verifies database exists before attempting operations
6. **Admin connection** - uses postgres admin database for operations

### Database Names

- **dev**: `soar_dev`
- **production**: `soar`

### Example Output

```
Database Reset Script
=====================
🔍 Running in DRY RUN mode - no changes will be made
Connecting to postgres admin database...
Checking if database 'soar_dev' exists...

🔍 DRY RUN MODE - No actual changes will be made
Would execute the following operations:
  1. Terminate all connections to 'soar_dev'
  2. DROP DATABASE soar_dev;
  3. CREATE DATABASE soar_dev;
```

### Post-Reset Setup

After running the reset script, you'll need to run migrations to recreate the schema:

```bash
# For dev environment
diesel migration run --database-url postgresql://user:pass@host:port/soar_dev

# For production environment
diesel migration run --database-url postgresql://user:pass@host:port/soar
```

### Installation

Install the required Python dependencies:

```bash
pip install psycopg[binary]
```

Or if using a virtual environment:

```bash
python -m pip install psycopg[binary]
```
