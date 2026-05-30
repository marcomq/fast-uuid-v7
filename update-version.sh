#!/bin/bash
# Script to update version across all project files
# Usage: ./update-version.sh <new-version>

set -e

if [ -z "$1" ]; then
    echo "Usage: $0 <new-version>"
    echo "Example: $0 0.1.5"
    exit 1
fi

NEW_VERSION="$1"

echo "Updating version to $NEW_VERSION..."

# Update root Cargo.toml workspace version
sed -i.bak "s/^version = \".*\"/version = \"$NEW_VERSION\"/" Cargo.toml && rm Cargo.toml.bak

echo "✓ Updated Cargo.toml"
echo "✓ python/Cargo.toml inherits version from workspace"
echo "✓ python/pyproject.toml reads version dynamically from Cargo.toml"

echo ""
echo "Version update complete! Now you only need to:"
echo "1. Update version in root Cargo.toml (line with 'version = \"...\")'"
echo "2. The other files will automatically use the same version"
echo ""
echo "Current version in Cargo.toml:"
grep "^version = " Cargo.toml | head -1

# Made with Bob
