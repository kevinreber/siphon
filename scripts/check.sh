#!/usr/bin/env bash
# Run all CI checks locally before committing
# Usage: ./scripts/check.sh

set -e

echo "🔍 Running all checks..."
echo ""

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

check_result() {
    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✓ $1 passed${NC}"
    else
        echo -e "${RED}✗ $1 failed${NC}"
        exit 1
    fi
}

# TypeScript CLI checks
echo "📦 Checking TypeScript CLI..."
cd siphon-cli

echo "  → Installing dependencies..."
npm install --silent

echo "  → Running type check..."
npm run typecheck
check_result "TypeScript type check"

echo "  → Running Biome linter..."
npx biome check src
check_result "Biome lint/format"

cd ..

# Rust daemon checks
echo ""
echo "🦀 Checking Rust daemon..."
cd siphon-daemon

echo "  → Running cargo check..."
cargo check --quiet
check_result "Cargo check"

echo "  → Running cargo fmt..."
cargo fmt --check
check_result "Cargo fmt"

echo "  → Running clippy..."
cargo clippy --quiet -- -W clippy::all
check_result "Clippy"

cd ..

echo ""
echo -e "${GREEN}✅ All checks passed!${NC}"
echo ""
echo "You can now commit your changes."
