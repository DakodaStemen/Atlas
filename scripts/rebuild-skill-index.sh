#!/usr/bin/env bash
# rebuild-skill-index.sh — Regenerate docs/SKILL_INDEX.md from skill file frontmatter.
# This is the canonical way to keep the index in sync. Run after any skill changes.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
SKILLS_DIR="$ROOT/skills"
INDEX_FILE="$ROOT/docs/SKILL_INDEX.md"

tmpfile=$(mktemp)
trap 'rm -f "$tmpfile"' EXIT

# Write header
cat > "$tmpfile" <<'HEADER'
# Skill Index

| Purpose | Path | Query | Domain |
|---------|------|-------|--------|
HEADER

# Process each skill file, sorted by directory then filename
while IFS= read -r f; do
    rel=$(realpath --relative-to="$ROOT" "$f")

    # Parse YAML frontmatter
    first_line=$(head -1 "$f")
    [[ "$first_line" != "---" ]] && continue

    fm_name=""
    fm_desc=""
    fm_domain=""
    fm_triggers=""
    fm_tags=""
    in_fm=0
    in_tags=0

    while IFS= read -r line; do
        if [[ "$in_fm" -eq 0 && "$line" == "---" ]]; then
            in_fm=1
            continue
        fi
        if [[ "$in_fm" -eq 1 && "$line" == "---" ]]; then
            break
        fi
        if [[ "$in_fm" -eq 1 ]]; then
            # If we were collecting multi-line tags and hit a non-continuation line, stop
            if [[ "$in_tags" -eq 1 ]] && [[ ! "$line" =~ ^[[:space:]]*- ]]; then
                in_tags=0
            fi

            if [[ "$line" =~ ^name:[[:space:]]*(.*) ]]; then
                fm_name="${BASH_REMATCH[1]}"
                # Strip surrounding quotes
                fm_name="${fm_name#\"}"
                fm_name="${fm_name%\"}"
                fm_name="${fm_name#\'}"
                fm_name="${fm_name%\'}"
            elif [[ "$line" =~ ^description:[[:space:]]*(.*) ]]; then
                fm_desc="${BASH_REMATCH[1]}"
                fm_desc="${fm_desc#\"}"
                fm_desc="${fm_desc%\"}"
                fm_desc="${fm_desc#\'}"
                fm_desc="${fm_desc%\'}"
            elif [[ "$line" =~ ^domain:[[:space:]]*(.*) ]]; then
                fm_domain="${BASH_REMATCH[1]}"
                fm_domain="${fm_domain#\"}"
                fm_domain="${fm_domain%\"}"
            elif [[ "$line" =~ ^triggers:[[:space:]]*(.*) ]]; then
                fm_triggers="${BASH_REMATCH[1]}"
                fm_triggers="${fm_triggers#\"}"
                fm_triggers="${fm_triggers%\"}"
            elif [[ "$line" =~ ^tags:[[:space:]]*\[(.*)\] ]]; then
                # Inline tags: [tag1, tag2, ...]
                fm_tags="${BASH_REMATCH[1]}"
            elif [[ "$line" =~ ^tags:[[:space:]]*$ ]]; then
                # Multi-line tags starting
                in_tags=1
                fm_tags=""
            elif [[ "$in_tags" -eq 1 && "$line" =~ ^[[:space:]]*-[[:space:]]*(.*) ]]; then
                tag="${BASH_REMATCH[1]}"
                tag="${tag#\"}"
                tag="${tag%\"}"
                if [[ -n "$fm_tags" ]]; then
                    fm_tags="$fm_tags, $tag"
                else
                    fm_tags="$tag"
                fi
            fi
        fi
    done < "$f"

    # Skip files without proper frontmatter
    [[ -z "$fm_name" || -z "$fm_desc" ]] && continue

    # Build the query column: prefer triggers, fall back to tags
    query="$fm_triggers"
    [[ -z "$query" ]] && query="$fm_tags"

    # Truncate description for the Purpose column if too long (keep it readable)
    purpose="$fm_desc"

    # Determine domain prefix for youtube skills
    dir_name=$(basename "$(dirname "$f")")
    if [[ "$dir_name" == "10-youtube" ]]; then
        purpose="**YT: ${fm_name#yt-}** — $fm_desc"
    fi

    echo "| $purpose | $rel | $query | $fm_domain |" >> "$tmpfile"

done < <(find "$SKILLS_DIR" -mindepth 2 -maxdepth 2 -name '*.md' -type f | sort)

# Replace the index file
mv "$tmpfile" "$INDEX_FILE"

count=$(tail -n +4 "$INDEX_FILE" | wc -l)
echo "Rebuilt $INDEX_FILE with $count entries."
