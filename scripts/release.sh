#!/usr/bin/env bash
set -euo pipefail

# Usage: ./scripts/release.sh <version>
# Example: ./scripts/release.sh 0.2.0
#
# This script:
# 1. Updates version numbers in Cargo.toml and package.json
# 2. Commits the version bump
# 3. Creates and pushes a git tag (v<version>)
# 4. The GitHub Actions release workflow handles the rest:
#    - Builds binaries for macOS (Intel + Apple Silicon) and Linux
#    - Creates a GitHub Release with the binaries
#    - Updates the Homebrew tap formula automatically

VERSION="${1:?Usage: ./scripts/release.sh <version>}"
TAG="v${VERSION}"

echo "Releasing siphon ${TAG}..."

# Ensure we're on main and clean
BRANCH=$(git rev-parse --abbrev-ref HEAD)
if [ "$BRANCH" != "main" ]; then
  echo "Warning: not on main branch (currently on ${BRANCH})"
  read -rp "Continue anyway? [y/N] " confirm
  [[ "$confirm" =~ ^[Yy]$ ]] || exit 1
fi

if ! git diff --quiet || ! git diff --cached --quiet; then
  echo "Error: working directory is not clean. Commit or stash changes first."
  exit 1
fi

# Update versions
echo "Updating version to ${VERSION}..."

# Cargo.toml
sed -i.bak "s/^version = \".*\"/version = \"${VERSION}\"/" siphon-daemon/Cargo.toml
rm -f siphon-daemon/Cargo.toml.bak

# package.json
cd siphon-cli
npm version "${VERSION}" --no-git-tag-version
cd ..

# Commit version bump
git add siphon-daemon/Cargo.toml siphon-cli/package.json siphon-cli/package-lock.json
git commit -m "chore: bump version to ${VERSION}"

# Create and push tag
git tag -a "${TAG}" -m "Release ${TAG}"
echo "Pushing tag ${TAG}..."
git push origin main
git push origin "${TAG}"

echo ""
echo "Release ${TAG} triggered!"
echo "Track progress at: https://github.com/kevinreber/siphon/actions"
echo ""
echo "Once the workflow completes:"
echo "  brew tap kevinreber/tap"
echo "  brew install siphon"
