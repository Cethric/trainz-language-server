#!/bin/bash
# Helper script for conventional commit messages
# Usage: ./create-commit.sh "type" "scope" "description"
# Example: ./create-commit.sh "feat" "parser" "add support for async functions"

set -e

if [ $# -lt 2 ]; then
    echo "Usage: ./create-commit.sh <type> [scope] <description>"
    echo ""
    echo "Types:"
    echo "  feat     - A new feature"
    echo "  fix      - A bug fix"
    echo "  perf     - Performance improvement"
    echo "  docs     - Documentation changes"
    echo "  style    - Code style changes"
    echo "  refactor - Code refactoring"
    echo "  test     - Test changes"
    echo "  ci       - CI/CD changes"
    echo "  chore    - Build/dependencies/tooling"
    echo ""
    echo "Scopes:"
    echo "  parser, lsp, ast, diagnostics, formatter, completions, hover, definition, vscode, jetbrains"
    echo ""
    echo "Examples:"
    echo "  ./create-commit.sh feat parser 'add support for async functions'"
    echo "  ./create-commit.sh fix lsp 'prevent deadlock in symbol indexing'"
    exit 1
fi

TYPE="$1"

# Determine if we have a scope
if [ $# -eq 2 ]; then
    # No scope provided, description is the second argument
    DESCRIPTION="$2"
    MESSAGE="$TYPE: $DESCRIPTION"
else
    # Scope provided
    SCOPE="$2"
    DESCRIPTION="$3"
    MESSAGE="$TYPE($SCOPE): $DESCRIPTION"
fi

echo "Commit message:"
echo "$MESSAGE"
echo ""
echo "Stage changes with: git add <files>"
echo "Then commit with: git commit -m '$MESSAGE'"
echo ""

# Optional: Show what would be staged
echo "Current git status:"
git status --short
