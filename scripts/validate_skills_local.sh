#!/usr/bin/env bash
# Local validation of skills: frontmatter (name, description), optional ## Purpose/When to use, max 50k chars.
# Does not replace MCP validate_skill; use for batch checks when server is not available.

set -euo pipefail

REPO_ROOT="${REPO_ROOT:-$(cd "$(dirname "$0")/.." && pwd)}"
SKILLS_DIR="${SKILLS_DIR:-$REPO_ROOT/skills}"
SAMPLE_SIZE="${SAMPLE_SIZE:-0}"
MAX_CHARS="${MAX_CHARS:-50000}"

valid=0
errors=0
total=0

# Collect files
mapfile -t all_files < <(find "$SKILLS_DIR" -maxdepth 1 -type f -name "*.md" ! -name "HOLLOW_SKILLS_README.md" | sort)

# Sample if requested
if (( SAMPLE_SIZE > 0 && SAMPLE_SIZE < ${#all_files[@]} )); then
    mapfile -t all_files < <(printf '%s\n' "${all_files[@]}" | shuf -n "$SAMPLE_SIZE")
fi

for f in "${all_files[@]}"; do
    ((total++))

    content=$(cat "$f" 2>/dev/null) || { ((errors++)); continue; }
    [[ -z "$content" ]] && { ((errors++)); continue; }

    char_count=${#content}
    if (( char_count > MAX_CHARS )); then
        ((errors++))
        continue
    fi

    has_name=$(echo "$content" | grep -cP '^(name|title):\s*.+' || true)
    has_desc=$(echo "$content" | grep -cP '^description:\s*.+' || true)

    if (( has_name == 0 || has_desc == 0 )); then
        ((errors++))
        continue
    fi

    ((valid++))
done

echo "Valid: $valid, Errors: $errors, Total: $total"
