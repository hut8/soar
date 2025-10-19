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

# Track validation results
ERRORS=0
WARNINGS=0

print_status "Validating SOAR development environment..."
echo ""

# Check Rust installation
if command_exists rustc; then
    RUST_VERSION=$(rustc --version)
    print_success "Rust installed: $RUST_VERSION"

    # Check for required components
    if rustup component list --installed | grep -q "rustfmt"; then
        print_success "rustfmt component installed"
    else
        print_error "rustfmt component missing. Run: rustup component add rustfmt"
        ERRORS=$((ERRORS + 1))
    fi

    if rustup component list --installed | grep -q "clippy"; then
        print_success "clippy component installed"
    else
        print_error "clippy component missing. Run: rustup component add clippy"
        ERRORS=$((ERRORS + 1))
    fi
else
    print_error "Rust not installed. Visit: https://rustup.rs/"
    ERRORS=$((ERRORS + 1))
fi

# Check Node.js installation
if command_exists node; then
    NODE_VERSION=$(node --version | cut -d'v' -f2)
    NODE_MAJOR=$(echo $NODE_VERSION | cut -d'.' -f1)

    if [ "$NODE_MAJOR" -ge 20 ]; then
        print_success "Node.js installed: v$NODE_VERSION"
    else
        print_warning "Node.js v$NODE_VERSION installed, but v20+ recommended"
        WARNINGS=$((WARNINGS + 1))
    fi
else
    print_error "Node.js not installed. Visit: https://nodejs.org/"
    ERRORS=$((ERRORS + 1))
fi

# Check npm
if command_exists npm; then
    NPM_VERSION=$(npm --version)
    print_success "npm installed: v$NPM_VERSION"
else
    print_error "npm not installed (should come with Node.js)"
    ERRORS=$((ERRORS + 1))
fi

# Check PostgreSQL
if command_exists psql; then
    PSQL_VERSION=$(psql --version | head -n1)
    print_success "PostgreSQL client installed: $PSQL_VERSION"

    # Test database connection if DATABASE_URL is set
    if [ -n "$DATABASE_URL" ]; then
        if psql "$DATABASE_URL" -c "SELECT version();" >/dev/null 2>&1; then
            print_success "Database connection successful"

            # Check for PostGIS
            if psql "$DATABASE_URL" -c "SELECT PostGIS_Version();" >/dev/null 2>&1; then
                print_success "PostGIS extension available"
            else
                print_warning "PostGIS extension not found in database"
                WARNINGS=$((WARNINGS + 1))
            fi
        else
            print_error "Cannot connect to database with DATABASE_URL"
            ERRORS=$((ERRORS + 1))
        fi
    else
        print_warning "DATABASE_URL not set. Set in .env file or environment"
        WARNINGS=$((WARNINGS + 1))
    fi
else
    print_error "PostgreSQL client (psql) not installed"
    ERRORS=$((ERRORS + 1))
fi

# Check Diesel CLI
if command_exists diesel; then
    DIESEL_VERSION=$(diesel --version)
    print_success "Diesel CLI installed: $DIESEL_VERSION"
else
    print_error "Diesel CLI not installed. Run: cargo install diesel_cli --no-default-features --features postgres"
    ERRORS=$((ERRORS + 1))
fi

# Check cargo tools
if command_exists cargo-audit; then
    print_success "cargo-audit installed"
else
    print_warning "cargo-audit not installed. Run: cargo install cargo-audit"
    WARNINGS=$((WARNINGS + 1))
fi

if cargo outdated --version >/dev/null 2>&1; then
    print_success "cargo-outdated installed"
else
    print_warning "cargo-outdated not installed. Run: cargo install cargo-outdated"
    WARNINGS=$((WARNINGS + 1))
fi

# Check pre-commit
if command_exists pre-commit; then
    print_success "pre-commit installed"

    # Check if hooks are installed
    if [ -f ".git/hooks/pre-commit" ]; then
        print_success "pre-commit hooks installed"
    else
        print_warning "pre-commit hooks not installed. Run: pre-commit install"
        WARNINGS=$((WARNINGS + 1))
    fi
else
    print_warning "pre-commit not installed. Run: ./scripts/setup-precommit.sh"
    WARNINGS=$((WARNINGS + 1))
fi

# Check NATS server
if command_exists nats-server; then
    print_success "NATS server installed"

    # Check if NATS is running
    if pgrep -f "nats-server" >/dev/null; then
        print_success "NATS server is running"
    else
        print_warning "NATS server installed but not running. Start with: nats-server &"
        WARNINGS=$((WARNINGS + 1))
    fi
else
    print_warning "NATS server not installed (optional for development)"
    WARNINGS=$((WARNINGS + 1))
fi

# Check environment files
if [ -f ".env" ]; then
    print_success ".env file exists"

    # Check for required variables
    if grep -q "DATABASE_URL" .env; then
        print_success "DATABASE_URL configured in .env"
    else
        print_warning "DATABASE_URL not found in .env"
        WARNINGS=$((WARNINGS + 1))
    fi
else
    print_error ".env file not found. Copy from .env.example"
    ERRORS=$((ERRORS + 1))
fi

# Check project files
if [ -f "web/package.json" ]; then
    print_success "Web project package.json exists"

    if [ -d "web/node_modules" ]; then
        print_success "Web dependencies installed"
    else
        print_warning "Web dependencies not installed. Run: cd web && npm ci"
        WARNINGS=$((WARNINGS + 1))
    fi
else
    print_error "Web project package.json not found"
    ERRORS=$((ERRORS + 1))
fi

if [ -f "Cargo.toml" ]; then
    print_success "Rust project Cargo.toml exists"
else
    print_error "Rust project Cargo.toml not found"
    ERRORS=$((ERRORS + 1))
fi

echo ""
print_status "Validation Summary:"

if [ $ERRORS -eq 0 ] && [ $WARNINGS -eq 0 ]; then
    print_success "Environment is fully configured! ✨"
    echo ""
    echo "You're ready to start development. Try:"
    echo "  cargo build"
    echo "  cd web && npm run build && cd .."
    echo "  pre-commit run --all-files"
elif [ $ERRORS -eq 0 ]; then
    print_success "Environment is ready with $WARNINGS warning(s)"
    echo ""
    echo "The warnings above are mostly optional. You can start development!"
else
    print_error "Found $ERRORS error(s) and $WARNINGS warning(s)"
    echo ""
    echo "Please fix the errors above before starting development."
    echo "Run ./scripts/install-dev-tools.sh to install missing tools."
    exit 1
fi
