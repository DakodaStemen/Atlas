#!/usr/bin/env bash
# Normalize an external SKILL.md to repo schema and write to skills/.
# Usage: ./normalize_skill.sh --input "path/to/SKILL.md" --skills-dir "./skills" [--skill-id "override-id"] [--source-repo "owner/repo"]

set -euo pipefail

MAX_CHARS=50000
INPUT_PATH=""
SKILLS_DIR=""
SKILL_ID=""
SOURCE_REPO=""
REPO_ROOT="${REPO_ROOT:-$(cd "$(dirname "$0")/.." && pwd)}"

# Parse arguments
while [[ $# -gt 0 ]]; do
    case "$1" in
        --input)       INPUT_PATH="$2"; shift 2 ;;
        --skills-dir)  SKILLS_DIR="$2"; shift 2 ;;
        --skill-id)    SKILL_ID="$2"; shift 2 ;;
        --source-repo) SOURCE_REPO="$2"; shift 2 ;;
        *) echo "Unknown option: $1" >&2; exit 1 ;;
    esac
done

if [[ -z "$INPUT_PATH" ]]; then
    echo "Error: --input is required" >&2
    exit 1
fi

[[ -z "$SKILLS_DIR" ]] && SKILLS_DIR="$REPO_ROOT/skills"

content=$(cat "$INPUT_PATH") || { echo "Error: Cannot read $INPUT_PATH" >&2; exit 1; }
if [[ -z "$content" ]]; then
    echo "Error: Empty file: $INPUT_PATH" >&2
    exit 1
fi

# Parse frontmatter
declare -A fm
body="$content"
has_frontmatter=0

if [[ "$content" == ---* ]]; then
    has_frontmatter=1
    # Extract frontmatter (between first and second ---)
    fm_block=$(echo "$content" | sed -n '2,/^---$/p' | head -n -1)
    body=$(echo "$content" | sed '1,/^---$/d' | sed '1,/^---$/d' 2>/dev/null || echo "$content" | awk 'BEGIN{c=0} /^---$/{c++; if(c==2){found=1; next}} found{print}')

    while IFS= read -r line; do
        if [[ "$line" =~ ^([a-zA-Z_]+):\ *(.*) ]]; then
            key="${BASH_REMATCH[1]}"
            val="${BASH_REMATCH[2]}"
            val=$(echo "$val" | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')
            fm[$key]="$val"
        fi
    done <<< "$fm_block"
fi

# Ensure name
name="${fm[name]:-}"
if [[ -z "$name" ]]; then
    name="${fm[title]:-}"
fi
if [[ -z "$name" ]]; then
    heading=$(echo "$body" | grep -m1 -oP '^#\s+\K.+' || true)
    name="${heading:-skill}"
fi

# Ensure description
desc="${fm[description]:-}"
if [[ -z "$desc" ]]; then
    desc=$(echo "$body" | head -3 | tr '\n' ' ')
fi
if [[ -z "$desc" || ${#desc} -lt 10 ]]; then
    desc="Use when the user needs guidance related to: $name."
fi

# Generate skill_id if not provided
if [[ -z "$SKILL_ID" ]]; then
    SKILL_ID=$(echo "$name" | sed 's/[^a-zA-Z0-9 -]//g' | tr '[:upper:]' '[:lower:]' | tr -s ' ' '-' | sed 's/^-\|-$//g')
    SKILL_ID="${SKILL_ID:0:64}"
    if [[ -z "$SKILL_ID" ]]; then
        SKILL_ID="skill-$(head -c 8 /dev/urandom | xxd -p | head -c 8)"
    fi
fi

# Build output with frontmatter
fm_out="---
name: $(echo "$name" | tr '\r\n' '  ')
description: $(echo "$desc" | tr '\r\n' '  ' | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')"

[[ -n "${fm[domain]:-}" ]]        && fm_out+=$'\n'"domain: ${fm[domain]}"
[[ -n "${fm[category]:-}" ]]      && fm_out+=$'\n'"category: ${fm[category]}"
[[ -n "${fm[triggers]:-}" ]]      && fm_out+=$'\n'"triggers: ${fm[triggers]}"
[[ -n "${fm[compatibility]:-}" ]] && fm_out+=$'\n'"compatibility: ${fm[compatibility]}"
[[ -n "${fm[tags]:-}" ]]          && fm_out+=$'\n'"tags: ${fm[tags]}"

fm_out+=$'\n'"---"$'\n\n'

# Add "When to use" section if missing
if ! echo "$body" | grep -q '##[[:space:]]*When to use'; then
    body=$(echo "$body" | sed 's/[[:space:]]*$//')
    body+=$'\n\n'"## When to use"$'\n\n'"Use when the user asks about or needs: $name."$'\n'
fi

out="${fm_out}${body}"

# Truncate if oversized
if (( ${#out} > MAX_CHARS )); then
    out="${out:0:$MAX_CHARS}"$'\n\n'"<!-- Truncated for size -->"
fi

# Write output
mkdir -p "$SKILLS_DIR"
out_path="$SKILLS_DIR/$SKILL_ID.md"
printf '%s' "$out" > "$out_path"

echo "$SKILL_ID"
if [[ -n "$SOURCE_REPO" ]]; then
    echo "SOURCE:$SOURCE_REPO"
fi
