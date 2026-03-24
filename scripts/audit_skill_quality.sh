#!/usr/bin/env bash
# Simple skill quality audit

set -euo pipefail

REPO_ROOT="${REPO_ROOT:-$(cd "$(dirname "$0")/.." && pwd)}"
SKILLS_DIR="${SKILLS_DIR:-$REPO_ROOT/skills}"
MAX_CHARS="${MAX_CHARS:-50000}"
MIN_DESC_LEN="${MIN_DESC_LEN:-20}"

fail_schema=()
short_desc=()
template_patterns=()
total=0

echo "=== Skill Quality Audit ==="
echo "Scanning: $SKILLS_DIR"

# Simple count
total=$(find "$SKILLS_DIR" -name "*.md" -type f ! -name "HOLLOW_SKILLS_README.md" | wc -l)
echo "Total skills found: $total"

# Check oversized files
oversized=$(find "$SKILLS_DIR" -name "*.md" -type f ! -name "HOLLOW_SKILLS_README.md" -exec wc -c {} + | awk '$1 > '$MAX_CHARS' {print $2}')
if [[ -n "$oversized" ]]; then
    echo "=== Oversized files (> $MAX_CHARS chars) ==="
    echo "$oversized"
fi

# Check for missing frontmatter
echo "=== Checking schema compliance ==="
missing_name=0
missing_desc=0
for f in $(find "$SKILLS_DIR" -name "*.md" -type f ! -name "HOLLOW_SKILLS_README.md" | head -10); do
    if ! grep -q "^name:" "$f" 2>/dev/null; then
        echo "$(basename "$f"): missing name"
        missing_name=$((missing_name + 1))
    fi
    if ! grep -q "^description:" "$f" 2>/dev/null; then
        echo "$(basename "$f"): missing description"
        missing_desc=$((missing_desc + 1))
    fi
done

echo "=== Summary ==="
echo "Total: $total"
echo "Missing name: $missing_name"
echo "Missing description: $missing_desc"
echo "Audit complete."
