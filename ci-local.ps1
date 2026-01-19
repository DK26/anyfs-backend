#!/usr/bin/env pwsh
# ci-local.ps1 - Local CI Test Runner for PowerShell
# Run all CI checks locally before pushing
# Usage: .\ci-local.ps1
#        .\ci-local.ps1 -Fix  # Auto-fix formatting issues

param(
    [switch]$Fix
)

$ErrorActionPreference = "Stop"

Write-Host "=== anyfs-backend Local CI ===" -ForegroundColor Cyan
Write-Host ""

# Check for cargo
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    $cargoPath = "$env:USERPROFILE\.cargo\bin"
    if (Test-Path "$cargoPath\cargo.exe") {
        $env:PATH = "$cargoPath;$env:PATH"
    } else {
        Write-Host "ERROR: cargo not found. Install Rust from https://rustup.rs" -ForegroundColor Red
        exit 1
    }
}

Write-Host "Using cargo: $(Get-Command cargo | Select-Object -ExpandProperty Source)" -ForegroundColor Green
$rustVersion = & rustc --version
Write-Host "Rust version: $rustVersion" -ForegroundColor Magenta

if ($rustVersion -match "nightly") {
    Write-Host "WARNING: Using nightly Rust. GitHub Actions uses stable." -ForegroundColor Yellow
}
Write-Host ""

function Run-Check {
    param([string]$Name, [string]$Command)
    
    Write-Host "Running: $Name" -ForegroundColor Blue
    Write-Host "  > $Command" -ForegroundColor Gray
    
    $startTime = Get-Date
    $output = $null
    try {
        # Use cmd /c to avoid PowerShell stderr handling issues
        $output = cmd /c "$Command 2>&1"
        $exitCode = $LASTEXITCODE
        if ($output) { Write-Host $output }
        if ($exitCode -ne 0) { throw "Command failed with exit code $exitCode" }
        $elapsed = ((Get-Date) - $startTime).TotalSeconds
        Write-Host "  PASSED ($([math]::Round($elapsed, 1))s)" -ForegroundColor Green
        Write-Host ""
    } catch {
        Write-Host "  FAILED: $_" -ForegroundColor Red
        exit 1
    }
}

function Run-Fix {
    param([string]$Name, [string]$Command)
    
    Write-Host "Auto-fixing: $Name" -ForegroundColor Yellow
    try {
        Invoke-Expression $Command 2>$null
    } catch {
        # Ignore fix failures
    }
}

# Check we're in the right directory
if (-not (Test-Path "Cargo.toml")) {
    Write-Host "ERROR: Must run from anyfs-backend root directory" -ForegroundColor Red
    exit 1
}

# Validate file encodings
Write-Host "Validating file encodings..." -ForegroundColor Cyan
$criticalFiles = @("README.md", "Cargo.toml")
foreach ($file in $criticalFiles) {
    if (Test-Path $file) {
        try {
            $content = Get-Content $file -Raw -Encoding UTF8
            Write-Host "  ${file}: UTF-8 OK" -ForegroundColor Green
        } catch {
            Write-Host "  ${file}: Encoding error!" -ForegroundColor Red
            exit 1
        }
    }
}
Write-Host ""

# Enforce code policy (no #[allow], no ignore in doctests)
Write-Host "Checking code policy..." -ForegroundColor Cyan
$violations = @()

Get-ChildItem -Path "src", "tests" -Recurse -Filter "*.rs" -ErrorAction SilentlyContinue | ForEach-Object {
    $content = Get-Content $_.FullName -Raw
    
    # Check for #[allow(...)]
    if ($content -match '#\s*\[\s*allow\s*\(') {
        $violations += "$($_.FullName): Contains #[allow(...)]"
    }
    
    # Check for ignore/no_run in doc tests
    if ($content -match '```\s*rust[^\n]*(ignore|no_run)') {
        $violations += "$($_.FullName): Contains ignore/no_run in doc test"
    }
}

if ($violations.Count -gt 0) {
    Write-Host "  Policy violations found:" -ForegroundColor Red
    $violations | ForEach-Object { Write-Host "    $_" -ForegroundColor Red }
    Write-Host ""
    Write-Host "Per AGENTS.md: All tests and doc examples MUST compile and run." -ForegroundColor Yellow
    exit 1
}
Write-Host "  No policy violations" -ForegroundColor Green
Write-Host ""

# Auto-fix if requested
if ($Fix) {
    Write-Host "Auto-fixing issues..." -ForegroundColor Cyan
    Run-Fix "Format" "cargo fmt --all"
    Run-Fix "Clippy auto-fix" "cargo clippy --fix --allow-dirty --allow-staged --all-targets --all-features 2>`$null"
    Run-Fix "Format after clippy" "cargo fmt --all"
    Write-Host ""
}

# Run CI checks
Write-Host "Running CI checks..." -ForegroundColor Cyan
Write-Host ""

Run-Check "Format Check" "cargo fmt --all -- --check"
Run-Check "Clippy (all features)" "cargo clippy --all-targets --all-features -- -D warnings"
Run-Check "Clippy (no features)" "cargo clippy --all-targets --no-default-features -- -D warnings"
Run-Check "Tests (all features)" "cargo test --all-features"
Run-Check "Tests (no features)" "cargo test --no-default-features"
Run-Check "Doc tests" "cargo test --doc --all-features"

$env:RUSTDOCFLAGS = "-D warnings"
Run-Check "Documentation" "cargo doc --no-deps --document-private-items --all-features"

Run-Check "Feature: none" "cargo check --no-default-features"
Run-Check "Feature: serde" "cargo check --no-default-features --features serde"
Run-Check "Feature: all" "cargo check --all-features"

# MSRV check (if rustup available)
if (Get-Command rustup -ErrorAction SilentlyContinue) {
    $msrvInstalled = $null
    try {
        $msrvInstalled = & rustup run 1.68.0 rustc --version 2>&1
        if ($LASTEXITCODE -eq 0) {
            Write-Host "Checking MSRV (1.68.0)..." -ForegroundColor Cyan
            Run-Check "MSRV Check" "rustup run 1.68.0 cargo check --all-features"
        } else {
            Write-Host "MSRV 1.68.0 not installed. Run: rustup install 1.68.0" -ForegroundColor Yellow
        }
    } catch {
        Write-Host "MSRV 1.68.0 not installed. Run: rustup install 1.68.0" -ForegroundColor Yellow
    }
} else {
    Write-Host "rustup not available. Skipping MSRV check." -ForegroundColor Yellow
}

# Security audit (if cargo-audit available)
if (Get-Command cargo-audit -ErrorAction SilentlyContinue) {
    Run-Check "Security Audit" "cargo audit"
} else {
    Write-Host "cargo-audit not installed. Run: cargo install cargo-audit" -ForegroundColor Yellow
}

Write-Host ""
Write-Host "=== All CI checks passed! ===" -ForegroundColor Green
Write-Host "Ready to push to remote." -ForegroundColor Green
