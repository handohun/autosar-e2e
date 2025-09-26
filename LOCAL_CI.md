# Local CI Setup Guide

This guide explains how to run CI checks locally before pushing to avoid remote failures.

## Quick Start

### 1. Make the script executable (Linux/macOS)
```bash
chmod +x local-ci.sh
```

### 2. Run all CI checks
```bash
./local-ci.sh
```

### 3. For Windows users
```cmd
# Using Git Bash (recommended)
./local-ci.sh

# Or using PowerShell/CMD
bash local-ci.sh
```

## What the Script Does

The `local-ci.sh` script runs the same checks as the GitHub Actions CI pipeline:

| Check | Command | Description |
|-------|---------|-------------|
| **Formatting** | `cargo fmt --all -- --check` | Ensures consistent code style |
| **Linting** | `cargo clippy --all-targets --all-features -- -D warnings` | Zero warnings tolerance |
| **Tests (Debug)** | `cargo test --verbose` | Runs all tests in debug mode |
| **Tests (Release)** | `cargo test --release --verbose` | Runs all tests in release mode |
| **Build (Debug)** | `cargo build --verbose` | Builds in debug mode |
| **Build (Release)** | `cargo build --release --verbose` | Builds in release mode |
| **Benchmarks** | `cargo bench --verbose` | Runs performance benchmarks |
| **Security Audit** | `cargo audit` | Scans for known vulnerabilities |
| **Coverage** | `cargo tarpaulin` | Generates code coverage report |

## Prerequisites

### Required (installed automatically with Rust)
- `cargo fmt`
- `cargo clippy`
- `cargo test`
- `cargo build`
- `cargo bench`

### Optional Tools
Install these for complete CI coverage:

```bash
# Security audit tool
cargo install cargo-audit

# Code coverage tool
cargo install cargo-tarpaulin
```

## Usage Examples

### Run full CI suite
```bash
./local-ci.sh
```

### Run individual checks
```bash
# Format check only
cargo fmt --all -- --check

# Linting only
cargo clippy --all-targets --all-features -- -D warnings

# Tests only
cargo test --verbose

# Benchmarks only
cargo bench --verbose
```

### Fix formatting issues
```bash
# Auto-fix formatting
cargo fmt --all

# Then check again
cargo fmt --all -- --check
```

## Recommended Workflow

1. **Before starting work:**
   ```bash
   git pull origin main
   ./local-ci.sh  # Ensure clean baseline
   ```

2. **During development:**
   ```bash
   # Run quick checks frequently
   cargo clippy --all-targets --all-features -- -D warnings
   cargo test
   ```

3. **Before committing:**
   ```bash
   ./local-ci.sh  # Full CI suite
   git add .
   git commit -m "Your commit message"
   ```

4. **Before pushing:**
   ```bash
   ./local-ci.sh  # Final check
   git push origin your-branch
   ```

## Troubleshooting

### Script permission denied (Linux/macOS)
```bash
chmod +x local-ci.sh
```

### Windows: bash command not found
- Install [Git for Windows](https://git-scm.com/download/win) (includes Git Bash)
- Or use WSL (Windows Subsystem for Linux)

### cargo-audit not found
```bash
cargo install cargo-audit
```

### cargo-tarpaulin not found (Linux only)
```bash
# Linux
cargo install cargo-tarpaulin

# macOS/Windows: coverage is optional
# The script will skip coverage if tarpaulin is not available
```

### Clippy warnings
Fix all clippy warnings - the CI has zero warnings tolerance:
```bash
cargo clippy --all-targets --all-features --fix -- -D warnings
```

### Test failures
```bash
# Run tests with more verbose output
cargo test -- --nocapture

# Run specific test
cargo test test_name -- --nocapture
```

## CI Pipeline Matching

The local script matches the GitHub Actions workflow in `.github/workflows/ci.yml`:
- Same rust toolchain (stable)
- Same clippy configuration
- Same test and build targets
- Same benchmark execution

## Performance Tips

- **Quick check:** Run `cargo clippy` and `cargo test` during development
- **Full check:** Run `./local-ci.sh` before commits
- **Parallel builds:** The script runs sequentially but you can run multiple terminals for faster feedback during development

## Integration with IDEs

### VS Code
Add to `.vscode/tasks.json`:
```json
{
    "label": "Local CI",
    "type": "shell",
    "command": "./local-ci.sh",
    "group": "build",
    "presentation": {
        "echo": true,
        "reveal": "always",
        "panel": "new"
    }
}
```

### Pre-commit Hook
Add to `.git/hooks/pre-commit`:
```bash
#!/bin/bash
exec ./local-ci.sh
```