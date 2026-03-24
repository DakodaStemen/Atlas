#!/usr/bin/env bash
# Truncate skills over 50k chars to meet validate_skill limit.

set -euo pipefail

REPO_ROOT="${REPO_ROOT:-$(cd "$(dirname "$0")/.." && pwd)}"
SKILLS_DIR="${SKILLS_DIR:-$REPO_ROOT/skills}"
MAX_CHARS="${MAX_CHARS:-50000}"

SUFFIX=$'\n<!-- Truncated for size; see source repo for full content -->'
SUFFIX_LEN=${#SUFFIX}
LIMIT=$((MAX_CHARS - SUFFIX_LEN))

FILES=(
    "express-nodejs-web-security-spec-express-5x-4192-nodejs-lts.md"
    "mapbox-mcp-runtime-patterns-mapbox-mapbox-agent-skills.md"
    "mapbox-mcp-runtime-patterns.md"
    "mapbox-search-integration.md"
    "react-best-practices.md"
    "react-native-skills.md"
)

for name in "${FILES[@]}"; do
    path="$SKILLS_DIR/$name"
    [[ -f "$path" ]] || continue

    char_count=$(wc -m < "$path")
    if (( char_count <= MAX_CHARS )); then
        continue
    fi

    # Truncate: take first LIMIT chars + suffix
    head -c "$LIMIT" "$path" > "${path}.tmp"
    printf '%s' "$SUFFIX" >> "${path}.tmp"
    mv "${path}.tmp" "$path"
    echo "Truncated: $name"
done
