#!/usr/bin/env bash
# Harvest skills from GitHub repos: clone, find SKILL.md or skill .md files, normalize, write to skills/, append SKILL_SOURCES.
# Usage: ./harvest_skills_from_github.sh --repo "owner/repo" [--branch main] [--sub-path ""] [--dry-run]

set -euo pipefail

# Defaults
REPO=""
BRANCH="main"
SUB_PATH=""
REPO_ROOT="${REPO_ROOT:-$(cd "$(dirname "$0")/.." && pwd)}"
SKILLS_DIR=""
SOURCES_PATH=""
TEMP_ROOT="${TMPDIR:-/tmp}/skills_harvest"
DRY_RUN=0

# Parse arguments
while [[ $# -gt 0 ]]; do
    case "$1" in
        --repo)       REPO="$2"; shift 2 ;;
        --branch)     BRANCH="$2"; shift 2 ;;
        --sub-path)   SUB_PATH="$2"; shift 2 ;;
        --skills-dir) SKILLS_DIR="$2"; shift 2 ;;
        --sources)    SOURCES_PATH="$2"; shift 2 ;;
        --dry-run)    DRY_RUN=1; shift ;;
        *) echo "Unknown option: $1"; exit 1 ;;
    esac
done

if [[ -z "$REPO" ]]; then
    echo "Error: --repo is required (e.g., --repo owner/repo)"
    exit 1
fi

[[ -z "$SKILLS_DIR" ]] && SKILLS_DIR="$REPO_ROOT/skills"
[[ -z "$SOURCES_PATH" ]] && SOURCES_PATH="$REPO_ROOT/docs/SKILL_SOURCES.md"
NORMALIZE_SCRIPT="$REPO_ROOT/scripts/normalize_skill.sh"

REPO_NAME="${REPO//\//-}"
CLONE_DIR="$TEMP_ROOT/$REPO_NAME"

# Clone or pull
if [[ ! -d "$CLONE_DIR" ]]; then
    echo "Cloning $REPO..."
    URL="https://github.com/$REPO.git"
    git clone --depth 1 -b "$BRANCH" "$URL" "$CLONE_DIR"
else
    pushd "$CLONE_DIR" > /dev/null
    git fetch --depth 1 origin "$BRANCH" 2>/dev/null || true
    git checkout "$BRANCH" 2>/dev/null || true
    popd > /dev/null
fi

SEARCH_ROOT="$CLONE_DIR"
if [[ -n "$SUB_PATH" ]]; then
    SEARCH_ROOT="$CLONE_DIR/$SUB_PATH"
fi
if [[ ! -d "$SEARCH_ROOT" ]]; then
    echo "Warning: SubPath not found: $SEARCH_ROOT"
    exit 0
fi

# Find candidate skill files
mapfile -t files < <(
    {
        find "$SEARCH_ROOT" -type f -name "SKILL.md" 2>/dev/null
        find "$SEARCH_ROOT" -type f -name "*.md" 2>/dev/null | while read -r md; do
            base=$(basename "$md")
            # Skip common non-skill files
            [[ "$base" == "README.md" ]] && continue
            [[ "$base" =~ ^(HOLLOW|CONTRIBUTING|CHANGELOG) ]] && continue
            # Check if it looks like a skill (has frontmatter or heading)
            head_lines=$(head -5 "$md" 2>/dev/null)
            if echo "$head_lines" | grep -qP '^---|^#\s+'; then
                echo "$md"
            fi
        done
    } | sort -u
)

# Build set of existing skill IDs
declare -A existing
if [[ -d "$SKILLS_DIR" ]]; then
    for ef in "$SKILLS_DIR"/*.md; do
        [[ -f "$ef" ]] || continue
        eid=$(basename "$ef" .md)
        existing[$eid]=1
    done
fi

added=0
source_lines=()

for f in "${files[@]}"; do
    rel="${f#$CLONE_DIR/}"

    # Normalize
    id=$("$NORMALIZE_SCRIPT" --input "$f" --skills-dir "$SKILLS_DIR" --source-repo "$REPO" 2>/dev/null | head -1) || {
        echo "  Warning: Normalize failed for $f"
        continue
    }
    [[ -z "$id" ]] && continue

    # Handle duplicates
    if [[ -n "${existing[$id]:-}" ]]; then
        suffix=$(echo "$REPO_NAME" | sed 's/[^a-z0-9-]//g' | sed 's/-\+/-/g')
        id="$id-$suffix"
        "$NORMALIZE_SCRIPT" --input "$f" --skills-dir "$SKILLS_DIR" --skill-id "$id" --source-repo "$REPO" > /dev/null 2>&1 || true
    fi

    existing[$id]=1
    if [[ $DRY_RUN -eq 0 ]]; then
        source_lines+=("| $id | harvested | $REPO | $REPO_NAME |")
    fi
    ((added++))
    echo "  $id <- $rel"
done

if [[ $DRY_RUN -eq 0 && ${#source_lines[@]} -gt 0 ]]; then
    printf '%s\n' "${source_lines[@]}" >> "$SOURCES_PATH"
fi

echo "Harvested $added skills from $REPO"
