# Database Scripts

This directory contains database management and utility scripts.

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