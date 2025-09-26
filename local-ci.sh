#!/bin/bash

# Local CI Script for autosar-e2e
# This script runs the same checks as the GitHub Actions CI pipeline locally
# to catch issues before pushing to remote repository.

set -e  # Exit on any error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_step() {
    echo -e "${BLUE}==>${NC} $1"
}

print_success() {
    echo -e "${GREEN}✓${NC} $1"
}

print_error() {
    echo -e "${RED}✗${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}⚠${NC} $1"
}

# Function to run a command with step description
run_step() {
    local description="$1"
    shift
    print_step "$description"
    if "$@"; then
        print_success "$description completed successfully"
    else
        print_error "$description failed"
        exit 1
    fi
    echo
}

echo -e "${BLUE}🚀 Running Local CI Checks for autosar-e2e${NC}"
echo "This will run the same checks as GitHub Actions CI pipeline"
echo "=================================================="
echo

# 1. Check formatting
run_step "Checking code formatting (rustfmt)" \
    cargo fmt --all -- --check

# 2. Run clippy (linting)
run_step "Running clippy linter (zero warnings tolerance)" \
    cargo clippy --all-targets --all-features -- -D warnings

# 3. Run tests (debug mode)
run_step "Running tests in debug mode" \
    cargo test --verbose

# 4. Run tests (release mode)
run_step "Running tests in release mode" \
    cargo test --release --verbose

# 5. Build (debug mode)
run_step "Building in debug mode" \
    cargo build --verbose

# 6. Build (release mode)
run_step "Building in release mode" \
    cargo build --release --verbose

# 7. Run benchmarks
run_step "Running benchmarks" \
    cargo bench --verbose

# 8. Security audit (optional - check if cargo-audit is installed)
if command -v cargo-audit >/dev/null 2>&1; then
    run_step "Running security audit" \
        cargo audit
else
    print_warning "cargo-audit not installed. Run 'cargo install cargo-audit' to enable security checks"
fi

# 9. Coverage (optional - check if cargo-tarpaulin is installed)
if command -v cargo-tarpaulin >/dev/null 2>&1; then
    print_step "Running coverage analysis (this may take a while...)"
    if cargo tarpaulin --verbose --all-features --workspace --timeout 120 --out xml; then
        print_success "Coverage analysis completed"
        if [ -f cobertura.xml ]; then
            echo "Coverage report saved to: cobertura.xml"
        fi
    else
        print_error "Coverage analysis failed"
        exit 1
    fi
else
    print_warning "cargo-tarpaulin not installed. Run 'cargo install cargo-tarpaulin' to enable coverage analysis"
fi

echo
echo -e "${GREEN}🎉 All CI checks passed successfully!${NC}"
echo -e "${GREEN}✓${NC} Your code is ready to push to remote repository"
echo

# Optional: Show git status
if git rev-parse --git-dir >/dev/null 2>&1; then
    echo -e "${BLUE}Git Status:${NC}"
    git status --short
fi