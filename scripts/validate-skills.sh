#!/usr/bin/env bash
# validate-skills.sh — Validate skill files, SKILL_INDEX.md, and agent rule sync.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

SKILLS_DIR="$ROOT/skills"
INDEX_FILE="$ROOT/docs/SKILL_INDEX.md"
CURSOR_AGENTS="$ROOT/.cursor/rules/agents"
WINDSURF_AGENTS="$ROOT/.windsurf/rules/agents"

FAIL=0
TOTAL_CHECKS=0
PASSED_CHECKS=0

pass() { TOTAL_CHECKS=$((TOTAL_CHECKS + 1)); PASSED_CHECKS=$((PASSED_CHECKS + 1)); echo "  PASS: $1"; }
fail() { TOTAL_CHECKS=$((TOTAL_CHECKS + 1)); FAIL=1; echo "  FAIL: $1"; }

# ─── Check 1: Valid frontmatter in all skill files ───────────────────────────
echo ""
echo "=== Check 1: Skill files have valid frontmatter (name: and description:) ==="
missing_fm=()
while IFS= read -r f; do
    # Read the file and check for YAML frontmatter delimiters
    has_name=0
    has_desc=0
    in_fm=0
    while IFS= read -r line; do
        if [[ "$in_fm" -eq 0 && "$line" == "---" ]]; then
            in_fm=1
            continue
        fi
        if [[ "$in_fm" -eq 1 && "$line" == "---" ]]; then
            break
        fi
        if [[ "$in_fm" -eq 1 ]]; then
            [[ "$line" =~ ^name: ]] && has_name=1
            [[ "$line" =~ ^description: ]] && has_desc=1
        fi
    done < "$f"
    if [[ "$has_name" -eq 0 || "$has_desc" -eq 0 ]]; then
        missing_fm+=("$(realpath --relative-to="$ROOT" "$f")")
    fi
done < <(find "$SKILLS_DIR" -mindepth 2 -maxdepth 2 -name '*.md' -type f | sort)

if [[ ${#missing_fm[@]} -eq 0 ]]; then
    pass "All skill files have name: and description: in frontmatter"
else
    fail "Missing frontmatter (name: or description:) in ${#missing_fm[@]} file(s):"
    for f in "${missing_fm[@]}"; do echo "       - $f"; done
fi

# ─── Check 2: Every skill in SKILL_INDEX.md exists as a file ─────────────────
echo ""
echo "=== Check 2: Every path in SKILL_INDEX.md exists as a file ==="
missing_files=()
while IFS= read -r line; do
    # Extract the Path column (second column) from table rows
    if [[ "$line" =~ ^\| ]] && [[ ! "$line" =~ ^\|[-] ]] && [[ ! "$line" =~ ^\|\ *Purpose ]]; then
        path=$(echo "$line" | awk -F'|' '{print $3}' | xargs)
        if [[ -n "$path" && ! -f "$ROOT/$path" ]]; then
            missing_files+=("$path")
        fi
    fi
done < "$INDEX_FILE"

if [[ ${#missing_files[@]} -eq 0 ]]; then
    pass "All paths in SKILL_INDEX.md point to existing files"
else
    fail "SKILL_INDEX.md references ${#missing_files[@]} missing file(s):"
    for f in "${missing_files[@]}"; do echo "       - $f"; done
fi

# ─── Check 3: Every skill file with frontmatter has an entry in SKILL_INDEX ──
echo ""
echo "=== Check 3: Every skill file with frontmatter is listed in SKILL_INDEX.md ==="
# Build set of paths from index
declare -A index_paths
while IFS= read -r line; do
    if [[ "$line" =~ ^\| ]] && [[ ! "$line" =~ ^\|[-] ]] && [[ ! "$line" =~ ^\|\ *Purpose ]]; then
        path=$(echo "$line" | awk -F'|' '{print $3}' | xargs)
        [[ -n "$path" ]] && index_paths["$path"]=1
    fi
done < "$INDEX_FILE"

unlisted=()
while IFS= read -r f; do
    # Check if file has frontmatter
    first_line=$(head -1 "$f")
    if [[ "$first_line" == "---" ]]; then
        rel=$(realpath --relative-to="$ROOT" "$f")
        if [[ -z "${index_paths[$rel]:-}" ]]; then
            unlisted+=("$rel")
        fi
    fi
done < <(find "$SKILLS_DIR" -mindepth 2 -maxdepth 2 -name '*.md' -type f | sort)

if [[ ${#unlisted[@]} -eq 0 ]]; then
    pass "All skill files with frontmatter are listed in SKILL_INDEX.md"
else
    fail "${#unlisted[@]} skill file(s) with frontmatter not in SKILL_INDEX.md:"
    for f in "${unlisted[@]}"; do echo "       - $f"; done
fi

# ─── Check 4: .cursor and .windsurf agent rules are in sync ──────────────────
echo ""
echo "=== Check 4: .cursor/rules/agents/ and .windsurf/rules/agents/ are in sync ==="
if [[ ! -d "$CURSOR_AGENTS" ]]; then
    fail ".cursor/rules/agents/ directory does not exist"
elif [[ ! -d "$WINDSURF_AGENTS" ]]; then
    fail ".windsurf/rules/agents/ directory does not exist"
else
    cursor_files=$(cd "$CURSOR_AGENTS" && ls -1 2>/dev/null | sort)
    windsurf_files=$(cd "$WINDSURF_AGENTS" && ls -1 2>/dev/null | sort)

    only_cursor=$(comm -23 <(echo "$cursor_files") <(echo "$windsurf_files"))
    only_windsurf=$(comm -13 <(echo "$cursor_files") <(echo "$windsurf_files"))

    if [[ -z "$only_cursor" && -z "$only_windsurf" ]]; then
        pass "Agent rule directories are in sync ($(echo "$cursor_files" | wc -l) files)"
    else
        if [[ -n "$only_cursor" ]]; then
            fail "Files only in .cursor/rules/agents/:"
            while IFS= read -r f; do echo "       - $f"; done <<< "$only_cursor"
        fi
        if [[ -n "$only_windsurf" ]]; then
            fail "Files only in .windsurf/rules/agents/:"
            while IFS= read -r f; do echo "       - $f"; done <<< "$only_windsurf"
        fi
    fi
fi

# ─── Summary ─────────────────────────────────────────────────────────────────
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Results: $PASSED_CHECKS/$TOTAL_CHECKS checks passed"
if [[ "$FAIL" -eq 0 ]]; then
    echo "Status: ALL PASS"
else
    echo "Status: SOME CHECKS FAILED"
fi
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

exit "$FAIL"
