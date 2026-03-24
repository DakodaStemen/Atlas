# UI Reasoning (Design Rules)

Simple rules for UI/UX decisions. Use with design tokens (Colors, Typography) for consistent spokes.

## Severity

- **Must:** Required for accessibility or correctness. Do not ship without.
- **Should:** Strong recommendation; avoid unless justified.
- **May:** Optional enhancement.

## Do

- Use design tokens (docs/design/data/) for colors and typography instead of hardcoded values.
- Prefer horizontal-first layouts (e.g. 3-column on desktop); avoid stacked-only layouts for dashboards.
- Run **verify_ui_integrity** (MCP) against DESIGN_AXIOMS for UI code before completion.
- Use **get_ui_blueprint** when generating dashboard or multi-column layouts.

## Do not

- Do not use low-contrast text (e.g. gray-400 on white) for primary content.
- Do not stack critical actions more than two levels deep without clear hierarchy.
- Do not ignore DESIGN_AXIOMS or design token files when generating UI.

## Decision rule

When in doubt: query_knowledge("DESIGN_AXIOMS design tokens UI") and apply the retrieved constraints.
