#!/usr/bin/env bash
# Build docs/SKILL_INDEX.md from skills/**/*.md (Purpose, Path, Query, Domain).

set -euo pipefail

REPO_ROOT="${1:-${REPO_ROOT:-$(cd "$(dirname "$0")/.." && pwd)}}"
SKILLS_DIR="${2:-$REPO_ROOT/skills}"
OUT_PATH="${3:-$REPO_ROOT/docs/SKILL_INDEX.md}"

# Build header
output="# Skill index

| Purpose | Path | Query | Domain |
|---------|------|-------|--------|
"

count=0
shopt -s globstar nullglob
for f in "$SKILLS_DIR"/**/*.md; do
    [[ -f "$f" ]] || continue
    base=$(basename "$f" .md)
    [[ "$base" == "HOLLOW_SKILLS_README" ]] && continue
    [[ "$base" == "README" ]] && continue

    content=$(cat "$f" 2>/dev/null) || continue
    [[ -z "$content" ]] && continue

    name="$base"
    purpose="$base"
    domain=""

    # Parse YAML frontmatter if present
    if [[ "$content" == ---* ]]; then
        # Extract frontmatter block (between first --- and second ---)
        fm=$(echo "$content" | sed -n '2,/^---$/p' | head -n -1)
        while IFS= read -r line; do
            if [[ "$line" =~ ^name:\ *(.*) ]]; then
                name="${BASH_REMATCH[1]}"
            fi
            if [[ "$line" =~ ^description:\ *(.*) ]]; then
                desc="${BASH_REMATCH[1]}"
                purpose="${desc:0:80}"
                purpose="${purpose//|/ }"
            fi
            if [[ "$line" =~ ^domain:\ *(.*) ]]; then
                domain="${BASH_REMATCH[1]}"
            fi
        done <<< "$fm"
    fi

    # Build query: alphanumeric, spaces, hyphens only
    query=$(echo "$name" | sed 's/[^a-zA-Z0-9 -]//g' | tr -s ' ')

    purpose="${purpose//|/ }"
    relpath="${f#$REPO_ROOT/}"
    output+="| $purpose | $relpath | $query | $domain |
"
    ((count++))
done

# Ensure output directory exists
mkdir -p "$(dirname "$OUT_PATH")"
echo "$output" > "$OUT_PATH"
echo "Wrote $OUT_PATH with $count entries."
