# justfile — valua development tasks
#
# Run `just` to see all available recipes.
# Run `just check` before every commit; it must pass.
#
# Install just: https://github.com/casey/just
#   macOS:    brew install just
#   Linux:    cargo install just
#   Windows:  cargo install just

# Default recipe lists all available commands.
default:
    @just --list

# ----------------------------------------------------------------------------
# Primary workflow targets
# ----------------------------------------------------------------------------

# Run all checks: format, lint, test, build. Must pass before every commit.
check: fmt-check clippy test build
    @echo "✓ All checks passed."

# Quick check during development. Skips release build for speed.
check-fast: fmt-check clippy test
    @echo "✓ Fast checks passed."

# CI entry point. Verbose, fails fast, includes coverage if available.
ci: fmt-check clippy-ci test-ci build-release
    @echo "✓ CI checks passed."

# ----------------------------------------------------------------------------
# Formatting
# ----------------------------------------------------------------------------

# Apply rustfmt to all crates.
fmt:
    cargo fmt --all

# Check formatting without modifying files. Used in CI and pre-commit.
fmt-check:
    cargo fmt --all -- --check

# ----------------------------------------------------------------------------
# Linting
# ----------------------------------------------------------------------------

# Run clippy with the project's lint level. Treats warnings as errors.
clippy:
    cargo clippy --all-targets --all-features -- -D warnings

# Stricter clippy used in CI. Includes pedantic lints.
clippy-ci:
    cargo clippy --all-targets --all-features -- \
        -D warnings \
        -W clippy::pedantic \
        -A clippy::module_name_repetitions \
        -A clippy::missing_errors_doc

# ----------------------------------------------------------------------------
# Testing
# ----------------------------------------------------------------------------

# Run all tests across the workspace.
test:
    cargo test --workspace --all-features

# Run tests with output streamed (useful for debugging a single failing test).
test-verbose:
    cargo test --workspace --all-features -- --nocapture

# Run tests with the CI configuration (single-threaded for deterministic output).
test-ci:
    cargo test --workspace --all-features -- --test-threads=1

# Run only the tests in a specific crate. Usage: just test-crate valua-parser
test-crate crate:
    cargo test -p {{crate}} --all-features

# Run only the integration tests (fixture-based).
test-integration:
    cargo test --workspace --test '*' --all-features

# Run a single test by name. Usage: just test-one bitwise_and
test-one name:
    cargo test --workspace {{name}} -- --nocapture

# ----------------------------------------------------------------------------
# Building
# ----------------------------------------------------------------------------

# Debug build of the workspace.
build:
    cargo build --workspace --all-features

# Release build of the CLI binary.
build-release:
    cargo build --release --bin valua

# ----------------------------------------------------------------------------
# Running the CLI
# ----------------------------------------------------------------------------

# Run `valua build` on a file. Usage: just run-build path/to/input.lua
run-build input:
    cargo run --bin valua -- build {{input}}

# Run `valua check` on a file. Usage: just run-check path/to/input.lua
run-check input:
    cargo run --bin valua -- check {{input}}

# Run `valua lint` on a file. Usage: just run-lint path/to/input.lua
run-lint input:
    cargo run --bin valua -- lint {{input}}

# ----------------------------------------------------------------------------
# Maintenance
# ----------------------------------------------------------------------------

# Remove all build artifacts.
clean:
    cargo clean

# Update dependencies within the constraints of Cargo.toml.
update:
    cargo update

# Check for outdated dependencies (requires cargo-outdated).
outdated:
    cargo outdated --workspace

# Audit dependencies for known security vulnerabilities (requires cargo-audit).
audit:
    cargo audit

# ----------------------------------------------------------------------------
# Documentation
# ----------------------------------------------------------------------------

# Build and open the workspace documentation.
docs:
    cargo doc --workspace --no-deps --open

# Build docs without opening (used in CI).
docs-build:
    cargo doc --workspace --no-deps

# ----------------------------------------------------------------------------
# Benchmarks
# ----------------------------------------------------------------------------

# Run all benchmarks.
bench:
    cargo bench --workspace

# Run a specific benchmark. Usage: just bench-one parser_throughput
bench-one name:
    cargo bench --workspace {{name}}

# ----------------------------------------------------------------------------
# Coverage (requires cargo-llvm-cov)
# ----------------------------------------------------------------------------

# Generate coverage report.
coverage:
    cargo llvm-cov --workspace --all-features --html

# Coverage report for CI (lcov format).
coverage-ci:
    cargo llvm-cov --workspace --all-features --lcov --output-path lcov.info

# ----------------------------------------------------------------------------
# Pre-commit and git hooks
# ----------------------------------------------------------------------------

# Install the pre-commit hook into .git/hooks/pre-commit.
install-hooks:
    @echo '#!/bin/sh' > .git/hooks/pre-commit
    @echo 'just check-fast || exit 1' >> .git/hooks/pre-commit
    @chmod +x .git/hooks/pre-commit
    @echo "✓ Pre-commit hook installed. It runs 'just check-fast' before each commit."

# ----------------------------------------------------------------------------
# Release helpers
# ----------------------------------------------------------------------------

# Verify the workspace is ready for a release: all checks pass, no uncommitted changes.
release-check: check
    @if [ -n "$(git status --porcelain)" ]; then \
        echo "✗ Working tree is not clean. Commit or stash changes before releasing."; \
        exit 1; \
    fi
    @echo "✓ Ready for release."

# Print the current workspace version.
version:
    @cargo pkgid -p valua-core | cut -d'#' -f2
