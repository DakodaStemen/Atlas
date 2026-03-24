#!/usr/bin/env python3
"""Fix broken YAML frontmatter in skill markdown files.

Handles:
- description fields starting with # or containing YAML-breaking chars
- tags/triggers arrays containing values with :, @, #, etc.
- Any unquoted values that break YAML parsing
"""

import os
import re
import yaml
import sys


def fix_yaml_value_in_list(match):
    """Quote problematic items inside YAML flow sequences [...] ."""
    inner = match.group(1)
    items = []
    for item in re.split(r',\s*', inner):
        item = item.strip()
        # If item contains chars that break YAML and isn't already quoted
        if item and not (item.startswith('"') and item.endswith('"')):
            if any(c in item for c in ':@#{}[]|>&*!%'):
                item = '"' + item.replace('"', '\\"') + '"'
        items.append(item)
    return '[' + ', '.join(items) + ']'


def fix_frontmatter(content):
    """Fix YAML frontmatter in a markdown file's content string."""
    if not content.startswith('---'):
        return content, False

    try:
        end_idx = content.index('---', 3)
    except ValueError:
        return content, False

    fm_text = content[3:end_idx]
    body = content[end_idx:]

    # Try parsing first - if it works, no fix needed
    try:
        yaml.safe_load(fm_text)
        return content, False
    except yaml.YAMLError:
        pass

    # Fix line by line
    fixed_lines = []
    changed = False
    for line in fm_text.splitlines():
        original = line

        # Match key: value lines
        m = re.match(r'^(\s*\w+):\s*(.+)$', line)
        if m:
            key_part = m.group(1)
            val_part = m.group(2).strip()

            # If value is a flow sequence [...]
            if val_part.startswith('[') and val_part.endswith(']'):
                inner = val_part[1:-1]
                fixed_val = fix_yaml_value_in_list(re.match(r'\[(.*)\]', val_part))
                if fixed_val != val_part:
                    line = f'{key_part}: {fixed_val}'
            # If value starts with # (looks like markdown header)
            elif val_part.startswith('#'):
                line = f'{key_part}: "{val_part}"'
            # If value contains problematic unquoted chars
            elif any(c in val_part for c in ':@#') and not (val_part.startswith('"') and val_part.endswith('"')):
                # Don't quote if it's a simple key: value (single colon separating key from value is fine)
                # But if the VALUE itself contains colons, quote it
                if not (val_part.startswith('[') or val_part.startswith('{')):
                    line = f'{key_part}: "{val_part}"'

        if line != original:
            changed = True
        fixed_lines.append(line)

    if not changed:
        return content, False

    fixed_fm = '\n'.join(fixed_lines)
    # Ensure newline before closing ---
    result = '---' + fixed_fm + '\n' + body

    # Verify the fix works
    try:
        yaml.safe_load(fixed_fm)
    except yaml.YAMLError as e:
        print(f"  WARNING: Fix did not resolve YAML error: {e}", file=sys.stderr)
        return content, False

    return result, True


def main():
    skills_dir = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), 'skills')
    fixed_count = 0
    error_count = 0

    for root, dirs, files in os.walk(skills_dir):
        for fname in sorted(files):
            if not fname.endswith('.md'):
                continue
            fpath = os.path.join(root, fname)

            with open(fpath, 'r', encoding='utf-8', errors='replace') as fh:
                content = fh.read()

            fixed_content, was_fixed = fix_frontmatter(content)

            if was_fixed:
                with open(fpath, 'w', encoding='utf-8') as fh:
                    fh.write(fixed_content)
                print(f"Fixed YAML: {fpath}")
                fixed_count += 1

    # Verify all files
    print(f"\n--- Verification ---")
    for root, dirs, files in os.walk(skills_dir):
        for fname in sorted(files):
            if not fname.endswith('.md'):
                continue
            fpath = os.path.join(root, fname)
            with open(fpath, 'r', encoding='utf-8', errors='replace') as fh:
                content = fh.read()
            if not content.startswith('---'):
                continue
            try:
                end = content.index('---', 3)
                fm = content[3:end]
                yaml.safe_load(fm)
            except Exception as e:
                print(f"STILL BROKEN: {fpath}: {e}")
                error_count += 1

    print(f"\nFixed: {fixed_count} files")
    print(f"Remaining errors: {error_count}")


if __name__ == '__main__':
    main()
