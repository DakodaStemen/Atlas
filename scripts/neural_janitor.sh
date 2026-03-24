#!/usr/bin/env bash
# Skill Janitor Script
# Purges duplicates, migrates skills to cortical columns, and rebuilds the index.

set -euo pipefail

REPO_ROOT="${REPO_ROOT:-$(cd "$(dirname "$0")/.." && pwd)}"
SKILLS_PATH="$REPO_ROOT/skills"
SCRIPTS_PATH="$REPO_ROOT/scripts"

echo -e "\033[36m--- Neural Janitor: Starting Sweep ---\033[0m"

# 1. Content-Hash Purge
echo "Step 1: Purging exact duplicates..."
declare -A hashes
deleted_count=0
while IFS= read -r -d '' file; do
    hash=$(md5sum "$file" | awk '{print $1}')
    if [[ -n "${hashes[$hash]:-}" ]]; then
        rm -f "$file"
        ((deleted_count++))
    else
        hashes[$hash]="$file"
    fi
done < <(find "$SKILLS_PATH" -type f -name "*.md" -print0 2>/dev/null)
echo "Purged $deleted_count duplicates."

# 2. Index Rebuild
echo "Step 2: Rebuilding Skill Index..."
if [[ -f "$SCRIPTS_PATH/build_skill_index.sh" ]]; then
    bash "$SCRIPTS_PATH/build_skill_index.sh"
else
    echo "  (build_skill_index.sh not found, skipping)"
fi

echo -e "\033[32m--- Neural Janitor: Sweep Complete ---\033[0m"
