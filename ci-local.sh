#!/bin/bash
# ci-local.sh - Local CI Test Runner
# Run all CI checks locally before pushing
# Usage: ./ci-local.sh
#        ./ci-local.sh --fix  # Auto-fix formatting issues

set -e

FIX=false
while [[ $# -gt 0 ]]; do
    case $1 in
        --fix|-f) FIX=true; shift ;;
        *) echo "Unknown option: $1"; exit 1 ;;
    esac
done

echo "=== anyfs-backend Local CI ==="
echo ""

# Check for cargo
if ! command -v cargo &> /dev/null; then
    if [ -f "$HOME/.cargo/bin/cargo" ]; then
        export PATH="$HOME/.cargo/bin:$PATH"
    else
        echo "ERROR: cargo not found. Install Rust from https://rustup.rs"
        exit 1
    fi
fi

echo "✓ Using cargo: $(command -v cargo)"
RUST_VERSION=$(rustc --version)
echo "🦀 Rust version: $RUST_VERSION"

if echo "$RUST_VERSION" | grep -q "nightly"; then
    echo "⚠️  WARNING: Using nightly Rust. GitHub Actions uses stable."
fi
echo ""

run_check() {
    local name="$1"
    local cmd="$2"
    
    echo "🔍 Running: $name"
    echo "   > $cmd"
    
    local start_time=$(date +%s)
    if eval "$cmd"; then
        local elapsed=$(($(date +%s) - start_time))
        echo "   ✅ PASSED (${elapsed}s)"
        echo ""
    else
        echo "   ❌ FAILED"
        exit 1
    fi
}

run_fix() {
    local name="$1"
    local cmd="$2"
    
    echo "🔧 Auto-fixing: $name"
    eval "$cmd" 2>/dev/null || true
}

# Check we're in the right directory
if [[ ! -f "Cargo.toml" ]]; then
    echo "ERROR: Must run from anyfs-backend root directory"
    exit 1
fi

# Validate file encodings
echo "🔍 Validating file encodings..."
for file in README.md Cargo.toml; do
    if [[ -f "$file" ]]; then
        if file "$file" | grep -qiE 'UTF-8|ASCII|text'; then
            echo "   ✅ $file: UTF-8 OK"
        else
            echo "   ❌ $file: Encoding error!"
            exit 1
        fi
    fi
done

# Check for BOM
if command -v xxd >/dev/null 2>&1 && head -c 3 README.md | xxd | grep -qi efbbbf; then
    echo "   ❌ README.md contains UTF-8 BOM"
    exit 1
fi
echo ""

# Enforce code policy
echo "🔎 Checking code policy..."
VIOLATIONS=""

# Check for #[allow(...)]
ALLOW_MATCHES=$(find src tests -name "*.rs" -exec grep -l '#\s*\[\s*allow\s*(' {} \; 2>/dev/null || true)
if [[ -n "$ALLOW_MATCHES" ]]; then
    VIOLATIONS="$VIOLATIONS\n#[allow(...)] found in: $ALLOW_MATCHES"
fi

# Check for ignore/no_run in doc tests
DOCTEST_MATCHES=$(find src tests -name "*.rs" -exec grep -l '```\s*rust.*\(ignore\|no_run\)' {} \; 2>/dev/null || true)
if [[ -n "$DOCTEST_MATCHES" ]]; then
    VIOLATIONS="$VIOLATIONS\nignore/no_run in doctests: $DOCTEST_MATCHES"
fi

if [[ -n "$VIOLATIONS" ]]; then
    echo -e "   ❌ Policy violations found:$VIOLATIONS"
    echo ""
    echo "Per AGENTS.md: All tests and doc examples MUST compile and run."
    exit 1
fi
echo "   ✅ No policy violations"
echo ""

# Auto-fix if requested
if $FIX; then
    echo "🔧 Auto-fixing issues..."
    run_fix "Format" "cargo fmt --all"
    run_fix "Clippy auto-fix" "cargo clippy --fix --allow-dirty --allow-staged --all-targets --all-features 2>/dev/null"
    run_fix "Format after clippy" "cargo fmt --all"
    echo ""
fi

# Run CI checks
echo "🦀 Running CI checks..."
echo ""

run_check "Format Check" "cargo fmt --all -- --check"
run_check "Clippy (all features)" "cargo clippy --all-targets --all-features -- -D warnings"
run_check "Clippy (no features)" "cargo clippy --all-targets --no-default-features -- -D warnings"
run_check "Tests (all features)" "cargo test --all-features"
run_check "Tests (no features)" "cargo test --no-default-features"
run_check "Doc tests" "cargo test --doc --all-features"

export RUSTDOCFLAGS="-D warnings"
run_check "Documentation" "cargo doc --no-deps --document-private-items --all-features"

run_check "Feature: none" "cargo check --no-default-features"
run_check "Feature: serde" "cargo check --no-default-features --features serde"
run_check "Feature: all" "cargo check --all-features"

# MSRV check (if rustup available)
if command -v rustup &> /dev/null; then
    if rustup run 1.68.0 rustc --version &> /dev/null; then
        echo "📋 Checking MSRV (1.68.0)..."
        run_check "MSRV Check" "rustup run 1.68.0 cargo check --all-features"
    else
        echo "⚠️  MSRV 1.68.0 not installed. Run: rustup install 1.68.0"
    fi
fi

# Security audit (if cargo-audit available)
if command -v cargo-audit &> /dev/null; then
    run_check "Security Audit" "cargo audit"
else
    echo "⚠️  cargo-audit not installed. Run: cargo install cargo-audit"
fi

echo ""
echo "🎉 All CI checks passed!"
echo "Ready to push to remote."
