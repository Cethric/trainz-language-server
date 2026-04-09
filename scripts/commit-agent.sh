#!/bin/bash

# CommitAgent verification and commit script
# This script ensures that all changes are valid before committing.
# It runs cargo fmt, cargo check, and cargo test.
# If all checks pass, it executes git commit with a co-author trailer.

set -e

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo "Starting CommitAgent verification..."

# 1. Check if the code is well-formatted
echo "Running cargo fmt --all -- --check..."
if ! cargo fmt --all -- --check; then
    echo -e "${RED}Error: Code is not well-formatted. Run 'cargo fmt --all' to fix.${NC}"
    exit 1
fi
echo -e "${GREEN}Formatting check passed.${NC}"

# 2. Check if the code builds
echo "Running cargo check..."
if ! cargo check; then
    echo -e "${RED}Error: Code does not build.${NC}"
    exit 1
fi
echo -e "${GREEN}Build check passed.${NC}"

# 3. Run all tests
echo "Running cargo test..."
if ! cargo test; then
    echo -e "${RED}Error: Some tests failed.${NC}"
    exit 1
fi
echo -e "${GREEN}All tests passed.${NC}"

# 4. Perform git commit if all checks passed
if [ -z "$1" ]; then
    echo -e "${RED}Error: No commit message provided.${NC}"
    echo "Usage: $0 \"Your commit message\""
    exit 1
fi

COMMIT_MSG="$1"
echo "All checks passed! Committing changes..."

# Using the co-author trailer as required by the guidelines
git commit -m "$COMMIT_MSG" --trailer "Co-authored-by: Junie <junie@jetbrains.com>"

echo -e "${GREEN}Changes committed successfully by CommitAgent!${NC}"
