#!/usr/bin/env bash
# check-freshness.sh — Report skill freshness and flag stale files / link-rot risk.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
SKILLS_DIR="$ROOT/skills"

STALE_DAYS=180
NOW=$(date +%s)

stale_count=0
url_count=0
total=0

echo ""
echo "=== Skill Freshness Report ==="
echo ""

# Collect data: file | days_since_modified | status | has_urls
entries=()
while IFS= read -r f; do
    total=$((total + 1))
    rel=$(realpath --relative-to="$ROOT" "$f")
    mod_epoch=$(stat -c %Y "$f" 2>/dev/null || stat -f %m "$f" 2>/dev/null)
    mod_date=$(date -d "@$mod_epoch" +%Y-%m-%d 2>/dev/null || date -r "$mod_epoch" +%Y-%m-%d 2>/dev/null)
    days_ago=$(( (NOW - mod_epoch) / 86400 ))

    status="OK"
    if [[ "$days_ago" -gt "$STALE_DAYS" ]]; then
        status="STALE"
        stale_count=$((stale_count + 1))
    fi

    has_urls=""
    if grep -qE 'https?://' "$f" 2>/dev/null; then
        has_urls="LINK-ROT-RISK"
        url_count=$((url_count + 1))
    fi

    entries+=("$days_ago|$rel|$mod_date|$status|$has_urls")
done < <(find "$SKILLS_DIR" -mindepth 2 -maxdepth 2 -name '*.md' -type f)

# Sort by days ago (most stale first)
IFS=$'\n' sorted=($(printf '%s\n' "${entries[@]}" | sort -t'|' -k1 -rn)); unset IFS

printf "%-70s %-12s %-6s %s\n" "FILE" "MODIFIED" "STATUS" "URLS"
printf "%-70s %-12s %-6s %s\n" "----" "--------" "------" "----"
for entry in "${sorted[@]}"; do
    IFS='|' read -r days rel mod_date status has_urls <<< "$entry"
    printf "%-70s %-12s %-6s %s\n" "$rel" "$mod_date (${days}d)" "$status" "$has_urls"
done

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "Total skills: $total"
echo "Stale (>$STALE_DAYS days): $stale_count"
echo "With external URLs (link-rot risk): $url_count"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
