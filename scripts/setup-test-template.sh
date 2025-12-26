#!/bin/bash
#
# Setup test template database for parallel test execution
#
# This script:
# 1. Drops the existing soar_test_template database (if it exists)
# 2. Creates a fresh soar_test_template database
# 3. Creates PostGIS extension
# 4. Runs all database migrations
# 5. Marks database as template (prevents accidental connections)
#
# The template database is used to create fast, isolated test databases
# for parallel test execution. Each test gets its own database created
# from this template using PostgreSQL's CREATE DATABASE ... TEMPLATE feature.
#
# Usage:
#   ./scripts/setup-test-template.sh
#
# Environment variables:
#   PGHOST - PostgreSQL host (default: localhost)
#   PGPORT - PostgreSQL port (default: 5432)
#   PGUSER - PostgreSQL user (default: postgres)
#   PGPASSWORD - PostgreSQL password (default: postgres)

set -e  # Exit on error

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default configuration
PGHOST="${PGHOST:-localhost}"
PGPORT="${PGPORT:-5432}"
PGUSER="${PGUSER:-postgres}"
PGPASSWORD="${PGPASSWORD:-postgres}"
TEMPLATE_DB_NAME="soar_test_template"

# Export for psql
export PGHOST PGPORT PGUSER PGPASSWORD

echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${BLUE}  SOAR Test Template Database Setup${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo -e "${YELLOW}Setting up template database: ${TEMPLATE_DB_NAME}${NC}"
echo "Host: ${PGHOST}:${PGPORT}"
echo "User: ${PGUSER}"
echo ""
echo "This template will be used to create isolated databases"
echo "for each test, enabling parallel test execution."
echo ""

# Step 1: Drop existing template database
echo -e "${YELLOW}[1/5] Dropping existing template database (if exists)...${NC}"
psql -d postgres -c "DROP DATABASE IF EXISTS ${TEMPLATE_DB_NAME} WITH (FORCE);" || {
    echo -e "${RED}Failed to drop template database${NC}"
    echo ""
    echo "This may fail if you're using PostgreSQL < 13."
    echo "Trying without WITH (FORCE)..."
    psql -d postgres -c "DROP DATABASE IF EXISTS ${TEMPLATE_DB_NAME};" || {
        echo -e "${RED}Failed to drop template database${NC}"
        exit 1
    }
}
echo -e "${GREEN}✓ Template database dropped${NC}"
echo ""

# Step 2: Create fresh template database
echo -e "${YELLOW}[2/5] Creating fresh template database...${NC}"
psql -d postgres -c "CREATE DATABASE ${TEMPLATE_DB_NAME};" || {
    echo -e "${RED}Failed to create template database${NC}"
    exit 1
}
echo -e "${GREEN}✓ Template database created${NC}"
echo ""

# Step 3: Create PostGIS extension
echo -e "${YELLOW}[3/5] Creating PostGIS extension...${NC}"
psql -d "${TEMPLATE_DB_NAME}" -c "CREATE EXTENSION IF NOT EXISTS postgis;" || {
    echo -e "${RED}Failed to create PostGIS extension${NC}"
    echo ""
    echo "Make sure PostGIS is installed:"
    echo "  Ubuntu/Debian: sudo apt-get install postgresql-<version>-postgis-3"
    echo "  macOS: brew install postgis"
    exit 1
}
echo -e "${GREEN}✓ PostGIS extension created${NC}"
echo ""

# Step 4: Run migrations on template
echo -e "${YELLOW}[4/5] Running database migrations on template...${NC}"
TEMPLATE_URL="postgres://${PGUSER}:${PGPASSWORD}@${PGHOST}:${PGPORT}/${TEMPLATE_DB_NAME}"

if command -v diesel &> /dev/null; then
    DATABASE_URL="${TEMPLATE_URL}" diesel migration run || {
        echo -e "${RED}Failed to run migrations${NC}"
        exit 1
    }
else
    echo -e "${YELLOW}diesel CLI not found, using cargo run...${NC}"
    DATABASE_URL="${TEMPLATE_URL}" cargo run --bin soar -- migrate || {
        echo -e "${RED}Failed to run migrations${NC}"
        exit 1
    }
fi
echo -e "${GREEN}✓ Migrations completed${NC}"
echo ""

# Step 5: Mark as template (prevents accidental connections)
echo -e "${YELLOW}[5/5] Marking database as template...${NC}"
psql -d postgres -c "UPDATE pg_database SET datistemplate = TRUE WHERE datname = '${TEMPLATE_DB_NAME}';" || {
    echo -e "${YELLOW}Warning: Could not mark database as template${NC}"
    echo "This is not critical, but prevents accidental connections to the template."
}
echo -e "${GREEN}✓ Database marked as template${NC}"
echo ""

echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${GREEN}✓ Test template database setup complete!${NC}"
echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo "Template database: ${TEMPLATE_DB_NAME}"
echo "Connection: ${TEMPLATE_URL}"
echo ""
echo -e "${GREEN}You can now run tests in parallel:${NC}"
echo "  cargo nextest run"
echo ""
echo -e "${YELLOW}Note:${NC} Re-run this script after adding new migrations"
echo "to update the template with the latest schema."
echo ""
