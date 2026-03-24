#!/usr/bin/env bash
# Proactive Rule Evolution: Read lessons_learned.md, identify repeating patterns,
# and propose or append high-signal rules to agent_rules.md.
# Triggers a RAG refresh after updating the rules.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

LESSONS_FILE="${1:-$REPO_ROOT/docs/lessons_learned.md}"
RULES_FILE="${2:-$REPO_ROOT/docs/agent_rules.md}"
RAG_BINARY="${3:-$REPO_ROOT/monolith/target/release/rag-mcp}"
ORT_LIB="$REPO_ROOT/monolith/lib/libonnxruntime.so.1.23.0"

echo -e "\033[36m============================================\033[0m"
echo -e "\033[36m  Proactive Rule Evolution  \033[0m"
echo "  Lessons: $LESSONS_FILE"
echo "  Rules  : $RULES_FILE"
echo -e "\033[36m============================================\033[0m"

if [[ ! -f "$LESSONS_FILE" ]]; then
    echo -e "\033[31mERROR: lessons_learned.md not found at: $LESSONS_FILE\033[0m"
    exit 1
fi

if [[ ! -f "$RULES_FILE" ]]; then
    echo -e "\033[33mRules file not found. Creating it...\033[0m"
    mkdir -p "$(dirname "$RULES_FILE")"
    cat > "$RULES_FILE" <<'REOF'
# Agent Rules

This file contains high-signal rules evolved from lessons learned.

## Rules
REOF
fi

LESSONS_CONTENT=$(cat "$LESSONS_FILE")

# ---------------------------------------------------------------------------
# RULE EVOLUTION LOGIC
# ---------------------------------------------------------------------------

echo "Analyzing lessons for repeating patterns..."

# Extract categories and count occurrences
declare -A cat_counts
while IFS= read -r match; do
    cat_name=$(echo "$match" | sed 's/^Category:[[:space:]]*//')
    cat_name=$(echo "$cat_name" | sed 's/[[:space:]]*$//')
    [[ -z "$cat_name" ]] && continue
    cat_counts[$cat_name]=$(( ${cat_counts[$cat_name]:-0} + 1 ))
done < <(echo "$LESSONS_CONTENT" | grep -oP 'Category:\s*\K\w+' || true)

num_categories=${#cat_counts[@]}

if (( num_categories > 0 )); then
    echo -e "\033[32mFound $num_categories lesson categories.\033[0m"
    # Sort by count descending, find top category
    top_cat=""
    top_count=0
    for cat in "${!cat_counts[@]}"; do
        count=${cat_counts[$cat]}
        echo "  - $cat ($count lessons)"
        if (( count > top_count )); then
            top_count=$count
            top_cat=$cat
        fi
    done
fi

# Generate rule proposal based on most frequent category
if (( num_categories > 0 )) && [[ -n "$top_cat" ]]; then
    today=$(date +%Y-%m-%d)
    echo -e "\033[36mProposing new rule for category '$top_cat'...\033[0m"
    cat >> "$RULES_FILE" <<EOF

### [Proposed Rule: $top_cat] Always verify pattern consistency for $top_cat tasks.

- **Reasoning:** Multiple lessons identified in the '$top_cat' category suggest a need for more rigorous verification.
- **Action:** Before completing a $top_cat task, run a consistency check against the existing documentation.
- **Source:** Proactive Rule Evolution ($today)
EOF
else
    today=$(date +%Y-%m-%d)
    echo -e "\033[33mNo clear patterns found yet. Appending generic evolution notice.\033[0m"
    cat >> "$RULES_FILE" <<EOF

### [Maintenance] Proactive Rule Evolution check completed on $today. No new rules proposed.
EOF
fi

# ---------------------------------------------------------------------------
# Trigger RAG refresh
# ---------------------------------------------------------------------------

if [[ ! -x "$RAG_BINARY" ]]; then
    echo -e "\033[33mWARNING: rag-mcp not found at $RAG_BINARY - skipping RAG refresh.\033[0m"
    exit 0
fi

echo -e "\033[36mTriggering RAG refresh for rules file...\033[0m"
export ORT_DYLIB_PATH="$ORT_LIB"
refresh_out=$("$RAG_BINARY" refresh-file-index -p "$RULES_FILE" 2>&1) || true
echo -e "\033[32mRAG refresh: $refresh_out\033[0m"

echo ""
echo -e "\033[32mEvolution complete.\033[0m"
echo ""
