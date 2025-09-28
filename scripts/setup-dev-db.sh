#!/bin/bash
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Function to check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Default configuration
DEFAULT_DB_NAME="soar_dev"
DEFAULT_TEST_DB_NAME="soar_test"
DEFAULT_USER="postgres"
DEFAULT_HOST="localhost"
DEFAULT_PORT="5432"

print_status "Setting up SOAR development databases..."

# Check if PostgreSQL is available
if ! command_exists psql; then
    print_error "PostgreSQL client (psql) not found. Please install PostgreSQL first."
    exit 1
fi

if ! command_exists createdb; then
    print_error "createdb command not found. Please install PostgreSQL client tools."
    exit 1
fi

# Get database configuration from user or use defaults
echo ""
print_status "Database Configuration"
echo "Press Enter to use default values in brackets"

read -p "Database host [$DEFAULT_HOST]: " DB_HOST
DB_HOST=${DB_HOST:-$DEFAULT_HOST}

read -p "Database port [$DEFAULT_PORT]: " DB_PORT
DB_PORT=${DB_PORT:-$DEFAULT_PORT}

read -p "Database user [$DEFAULT_USER]: " DB_USER
DB_USER=${DB_USER:-$DEFAULT_USER}

read -s -p "Database password (leave empty if not required): " DB_PASSWORD
echo ""

read -p "Development database name [$DEFAULT_DB_NAME]: " DB_NAME
DB_NAME=${DB_NAME:-$DEFAULT_DB_NAME}

read -p "Test database name [$DEFAULT_TEST_DB_NAME]: " TEST_DB_NAME
TEST_DB_NAME=${TEST_DB_NAME:-$DEFAULT_TEST_DB_NAME}

# Build connection parameters
PSQL_ARGS="-h $DB_HOST -p $DB_PORT -U $DB_USER"

if [ -n "$DB_PASSWORD" ]; then
    export PGPASSWORD="$DB_PASSWORD"
    DATABASE_URL_AUTH="$DB_USER:$DB_PASSWORD"
else
    DATABASE_URL_AUTH="$DB_USER"
fi

echo ""
print_status "Creating databases..."

# Create development database
print_status "Creating development database: $DB_NAME"
if createdb $PSQL_ARGS $DB_NAME 2>/dev/null; then
    print_success "Development database '$DB_NAME' created"
else
    print_warning "Database '$DB_NAME' may already exist (this is OK)"
fi

# Create test database
print_status "Creating test database: $TEST_DB_NAME"
if createdb $PSQL_ARGS $TEST_DB_NAME 2>/dev/null; then
    print_success "Test database '$TEST_DB_NAME' created"
else
    print_warning "Database '$TEST_DB_NAME' may already exist (this is OK)"
fi

# Install extensions
print_status "Installing PostgreSQL extensions..."

# PostGIS extension for development database
print_status "Installing PostGIS in development database..."
if psql $PSQL_ARGS -d $DB_NAME -c "CREATE EXTENSION IF NOT EXISTS postgis;" >/dev/null 2>&1; then
    print_success "PostGIS extension installed in development database"
else
    print_error "Failed to install PostGIS extension in development database"
    print_status "Make sure PostGIS is installed: sudo apt-get install postgresql-contrib postgis"
fi

# pg_trgm extension for full-text search
print_status "Installing pg_trgm in development database..."
if psql $PSQL_ARGS -d $DB_NAME -c "CREATE EXTENSION IF NOT EXISTS pg_trgm;" >/dev/null 2>&1; then
    print_success "pg_trgm extension installed in development database"
else
    print_warning "Failed to install pg_trgm extension (this may not be critical)"
fi

# PostGIS extension for test database
print_status "Installing PostGIS in test database..."
if psql $PSQL_ARGS -d $TEST_DB_NAME -c "CREATE EXTENSION IF NOT EXISTS postgis;" >/dev/null 2>&1; then
    print_success "PostGIS extension installed in test database"
else
    print_error "Failed to install PostGIS extension in test database"
fi

# pg_trgm extension for test database
print_status "Installing pg_trgm in test database..."
if psql $PSQL_ARGS -d $TEST_DB_NAME -c "CREATE EXTENSION IF NOT EXISTS pg_trgm;" >/dev/null 2>&1; then
    print_success "pg_trgm extension installed in test database"
else
    print_warning "Failed to install pg_trgm extension in test database"
fi

# Generate DATABASE_URLs
DEV_DATABASE_URL="postgres://$DATABASE_URL_AUTH@$DB_HOST:$DB_PORT/$DB_NAME"
TEST_DATABASE_URL="postgres://$DATABASE_URL_AUTH@$DB_HOST:$DB_PORT/$TEST_DB_NAME"

# Update .env file or create it
print_status "Updating .env file..."

if [ -f ".env" ]; then
    # Update existing .env file
    if grep -q "^DATABASE_URL=" .env; then
        sed -i.bak "s|^DATABASE_URL=.*|DATABASE_URL=\"$DEV_DATABASE_URL\"|" .env
        print_status "Updated DATABASE_URL in .env"
    else
        echo "DATABASE_URL=\"$DEV_DATABASE_URL\"" >> .env
        print_status "Added DATABASE_URL to .env"
    fi
else
    # Create new .env file from example
    if [ -f ".env.example" ]; then
        cp .env.example .env
        if grep -q "^DATABASE_URL=" .env; then
            sed -i.bak "s|^DATABASE_URL=.*|DATABASE_URL=\"$DEV_DATABASE_URL\"|" .env
        else
            echo "DATABASE_URL=\"$DEV_DATABASE_URL\"" >> .env
        fi
        print_success "Created .env file from .env.example"
    else
        echo "DATABASE_URL=\"$DEV_DATABASE_URL\"" > .env
        print_success "Created new .env file"
    fi
fi

# Run database migrations if Diesel is available
if command_exists diesel; then
    print_status "Running database migrations..."

    # Set environment for migrations
    export DATABASE_URL="$DEV_DATABASE_URL"

    if diesel migration run; then
        print_success "Development database migrations completed"
    else
        print_error "Failed to run migrations on development database"
        print_status "You may need to run 'diesel migration run' manually"
    fi

    # Run migrations on test database
    export DATABASE_URL="$TEST_DATABASE_URL"
    if diesel migration run; then
        print_success "Test database migrations completed"
    else
        print_error "Failed to run migrations on test database"
    fi
else
    print_warning "Diesel CLI not found. Install it and run migrations manually:"
    echo "  cargo install diesel_cli --no-default-features --features postgres"
    echo "  DATABASE_URL=\"$DEV_DATABASE_URL\" diesel migration run"
fi

echo ""
print_success "Database setup complete!"
echo ""
echo "Configuration:"
echo "  Development DB: $DB_NAME"
echo "  Test DB: $TEST_DB_NAME"
echo "  Host: $DB_HOST:$DB_PORT"
echo "  User: $DB_USER"
echo ""
echo "Environment variables:"
echo "  Development: DATABASE_URL=\"$DEV_DATABASE_URL\""
echo "  Test: DATABASE_URL=\"$TEST_DATABASE_URL\""
echo ""
echo "The development DATABASE_URL has been saved to .env"
echo "Use the test DATABASE_URL for running tests."
echo ""
echo "Next steps:"
echo "1. Verify the setup: ./scripts/validate-env.sh"
echo "2. Build the project: cargo build"
echo "3. Run tests: cargo test"

# Clean up password from environment
unset PGPASSWORD