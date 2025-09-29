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

print_status "Installing SOAR development tools..."

# Check and install Rust toolchain components
if command_exists rustc; then
    print_status "Rust is installed ($(rustc --version))"

    # Install required Rust components
    print_status "Installing/updating Rust toolchain components..."
    rustup component add rustfmt clippy
    print_success "Rust components (rustfmt, clippy) installed/updated"
else
    print_error "Rust is not installed. Please install Rust first:"
    echo "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

# Check Node.js version
if command_exists node; then
    NODE_VERSION=$(node --version | cut -d'v' -f2)
    NODE_MAJOR=$(echo $NODE_VERSION | cut -d'.' -f1)

    if [ "$NODE_MAJOR" -ge 20 ]; then
        print_status "Node.js is installed (v$NODE_VERSION) ✓"
    else
        print_warning "Node.js version $NODE_VERSION is installed, but v20+ is recommended"
        print_status "Consider upgrading: https://nodejs.org/"
    fi
else
    print_error "Node.js is not installed. Please install Node.js 20+ first:"
    echo "  Visit https://nodejs.org/ or use a version manager like nvm"
    exit 1
fi

# Check npm
if command_exists npm; then
    print_status "npm is installed ($(npm --version))"
else
    print_error "npm is not installed. It should come with Node.js."
    exit 1
fi

# Install Diesel CLI
if command_exists diesel; then
    print_status "Diesel CLI is already installed ($(diesel --version))"
else
    print_status "Installing Diesel CLI..."
    if cargo install diesel_cli --no-default-features --features postgres; then
        print_success "Diesel CLI installed successfully"
    else
        print_error "Failed to install Diesel CLI. You may need to install PostgreSQL development libraries:"
        echo "  Ubuntu/Debian: sudo apt-get install libpq-dev"
        echo "  macOS: brew install postgresql"
        echo "  Then retry: cargo install diesel_cli --no-default-features --features postgres"
    fi
fi

# Install cargo-audit
if command_exists cargo-audit; then
    print_status "cargo-audit is already installed"
else
    print_status "Installing cargo-audit..."
    if cargo install cargo-audit; then
        print_success "cargo-audit installed successfully"
    else
        print_error "Failed to install cargo-audit"
    fi
fi

# Install cargo-outdated
if cargo outdated --version >/dev/null 2>&1; then
    print_status "cargo-outdated is already installed"
else
    print_status "Installing cargo-outdated..."
    if cargo install cargo-outdated; then
        print_success "cargo-outdated installed successfully"
    else
        print_error "Failed to install cargo-outdated"
    fi
fi

# Check PostgreSQL
if command_exists psql; then
    print_status "PostgreSQL client is installed ($(psql --version | head -n1))"
else
    print_warning "PostgreSQL client (psql) is not installed."
    echo "  Ubuntu/Debian: sudo apt-get install postgresql-client"
    echo "  macOS: brew install postgresql"
fi

# Install web dependencies if package.json exists
if [ -f "web/package.json" ]; then
    print_status "Installing web dependencies..."
    cd web
    if npm ci; then
        print_success "Web dependencies installed successfully"
    else
        print_error "Failed to install web dependencies"
        cd ..
        exit 1
    fi

    # Install Playwright browsers
    print_status "Installing Playwright browsers..."
    if npx playwright install chromium --with-deps; then
        print_success "Playwright browsers installed successfully"
    else
        print_warning "Failed to install Playwright browsers. Tests may not work."
    fi
    cd ..
else
    print_warning "web/package.json not found. Skipping web dependencies."
fi

echo ""
print_success "Development tools installation complete!"
echo ""
echo "Installed tools:"
echo "  ✓ Rust toolchain (rustfmt, clippy)"
echo "  ✓ Diesel CLI (database migrations)"
echo "  ✓ cargo-audit (security auditing)"
echo "  ✓ cargo-outdated (dependency checking)"
echo "  ✓ Web dependencies (if web/package.json exists)"
echo "  ✓ Playwright browsers (for E2E tests)"
echo ""
echo "Next steps:"
echo "1. Set up PostgreSQL database (see CONTRIBUTING.md)"
echo "2. Copy .env.example to .env and configure"
echo "3. Run 'diesel migration run' to set up database schema"
echo "4. Run 'scripts/setup-precommit.sh' to install pre-commit hooks"