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

# Check if we're in the project root
if [ ! -f ".pre-commit-config.yaml" ]; then
    print_error "This script must be run from the project root directory (where .pre-commit-config.yaml exists)"
    exit 1
fi

print_status "Setting up pre-commit hooks for SOAR development..."

# Check if Python is available
if ! command -v python3 &> /dev/null; then
    print_error "Python 3 is required but not installed. Please install Python 3 first."
    exit 1
fi

# Check if pip is available
if ! command -v pip3 &> /dev/null; then
    print_error "pip3 is required but not installed. Please install pip3 first."
    exit 1
fi

# Install pre-commit if not already installed
if ! command -v pre-commit &> /dev/null; then
    print_status "Installing pre-commit..."
    if command -v pipx &> /dev/null; then
        pipx install pre-commit
    else
        pip3 install --user pre-commit
        # Add pip user bin to PATH if not already there
        if [[ ":$PATH:" != *":$HOME/.local/bin:"* ]]; then
            print_warning "Adding ~/.local/bin to PATH for this session"
            export PATH="$HOME/.local/bin:$PATH"
        fi
    fi
    print_success "pre-commit installed successfully"
else
    print_status "pre-commit is already installed"
fi

# Install the git hook scripts
print_status "Installing pre-commit hooks..."
pre-commit install

# Install commit-msg hook for additional validation
pre-commit install --hook-type commit-msg

print_success "Pre-commit hooks installed successfully!"

# Run install-dev-tools.sh if it exists
if [ -f "scripts/install-dev-tools.sh" ]; then
    print_status "Installing development tools..."
    bash scripts/install-dev-tools.sh
else
    print_warning "Development tools script not found. You may need to install required tools manually:"
    echo "  - Rust toolchain (rustfmt, clippy)"
    echo "  - Node.js 20+ and npm"
    echo "  - PostgreSQL with PostGIS"
    echo "  - Diesel CLI: cargo install diesel_cli --no-default-features --features postgres"
    echo "  - cargo-audit: cargo install cargo-audit"
    echo "  - cargo-outdated: cargo install cargo-outdated"
fi

# Test the installation by running pre-commit on all files
print_status "Testing pre-commit installation..."
if pre-commit run --all-files --show-diff-on-failure; then
    print_success "All pre-commit checks passed!"
else
    print_warning "Some pre-commit checks failed. This is normal for initial setup."
    print_status "Run the following to fix auto-fixable issues:"
    echo "  pre-commit run --all-files"
fi

echo ""
print_success "Pre-commit setup complete!"
echo ""
echo "Next steps:"
echo "1. Set up your local database (see CONTRIBUTING.md for details)"
echo "2. Copy .env.example to .env and configure your environment"
echo "3. Run 'diesel migration run' to set up the database schema"
echo "4. Try making a commit - pre-commit will run automatically!"
echo ""
echo "To run pre-commit manually on all files:"
echo "  pre-commit run --all-files"
echo ""
echo "To run pre-commit on specific files:"
echo "  pre-commit run --files src/main.rs web/src/app.js"