#!/bin/bash
#
# Setup test database for E2E and integration tests
#
# This script:
# 1. Drops the existing soar_test database (if it exists)
# 2. Creates a fresh soar_test database
# 3. Runs all database migrations
# 4. Seeds test data using the seed-test-data command
#
# Usage:
#   ./scripts/setup-test-db.sh
#
# Environment variables:
#   DATABASE_URL - Full database URL (default: postgres://postgres:postgres@localhost:5432/soar_test)
#   PGHOST - PostgreSQL host (default: localhost)
#   PGPORT - PostgreSQL port (default: 5432)
#   PGUSER - PostgreSQL user (default: postgres)
#   PGPASSWORD - PostgreSQL password (default: postgres)
#   TEST_USER_EMAIL - Test user email (default: test@example.com)
#   TEST_USER_PASSWORD - Test user password (default: testpassword123)
#   SEED_COUNT - Number of fake records to create (default: 10)

set -e  # Exit on error

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Default configuration
PGHOST="${PGHOST:-localhost}"
PGPORT="${PGPORT:-5432}"
PGUSER="${PGUSER:-postgres}"
PGPASSWORD="${PGPASSWORD:-postgres}"
DB_NAME="soar_test"

# Database URL for migrations and seeding
export DATABASE_URL="${DATABASE_URL:-postgres://${PGUSER}:${PGPASSWORD}@${PGHOST}:${PGPORT}/${DB_NAME}}"

echo -e "${YELLOW}Setting up test database: ${DB_NAME}${NC}"
echo "Host: ${PGHOST}:${PGPORT}"
echo "User: ${PGUSER}"
echo ""

# Step 1: Drop existing database
echo -e "${YELLOW}[1/4] Dropping existing database (if exists)...${NC}"
psql -h "${PGHOST}" -p "${PGPORT}" -U "${PGUSER}" -d postgres -c "DROP DATABASE IF EXISTS ${DB_NAME};" || {
    echo -e "${RED}Failed to drop database${NC}"
    exit 1
}
echo -e "${GREEN}✓ Database dropped${NC}"
echo ""

# Step 2: Create fresh database
echo -e "${YELLOW}[2/4] Creating fresh database...${NC}"
psql -h "${PGHOST}" -p "${PGPORT}" -U "${PGUSER}" -d postgres -c "CREATE DATABASE ${DB_NAME};" || {
    echo -e "${RED}Failed to create database${NC}"
    exit 1
}
echo -e "${GREEN}✓ Database created${NC}"
echo ""

# Step 3: Create PostGIS extension
echo -e "${YELLOW}[3/4] Creating PostGIS extension...${NC}"
psql -h "${PGHOST}" -p "${PGPORT}" -U "${PGUSER}" -d "${DB_NAME}" -c "CREATE EXTENSION IF NOT EXISTS postgis;" || {
    echo -e "${RED}Failed to create PostGIS extension${NC}"
    exit 1
}
echo -e "${GREEN}✓ PostGIS extension created${NC}"
echo ""

# Step 4: Run migrations
echo -e "${YELLOW}[4/4] Running database migrations...${NC}"
if command -v diesel &> /dev/null; then
    diesel migration run || {
        echo -e "${RED}Failed to run migrations${NC}"
        exit 1
    }
else
    echo -e "${YELLOW}diesel CLI not found, using cargo run...${NC}"
    cargo run --bin soar -- migrate || {
        echo -e "${RED}Failed to run migrations${NC}"
        exit 1
    }
fi
echo -e "${GREEN}✓ Migrations completed${NC}"
echo ""

# Step 5: Seed test data
echo -e "${YELLOW}[5/5] Seeding test data...${NC}"
cargo run --bin soar -- seed-test-data || {
    echo -e "${RED}Failed to seed test data${NC}"
    exit 1
}
echo -e "${GREEN}✓ Test data seeded${NC}"
echo ""

echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${GREEN}✓ Test database setup complete!${NC}"
echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
echo "Database: ${DB_NAME}"
echo "Connection: ${DATABASE_URL}"
echo ""
echo "Test credentials:"
echo "  Email: ${TEST_USER_EMAIL:-test@example.com}"
echo "  Password: ${TEST_USER_PASSWORD:-testpassword123}"
echo ""
echo "You can now run E2E tests with:"
echo "  cd web && npm test"
