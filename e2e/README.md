# E2E tests (Playwright)

Minimal Playwright suite for the Audit Gate. The agent runs these as part of the Closed-Loop Audit when the task is UI/frontend-related.

**This repo does not include a frontend app by default.** Do **not** run `npm run test:e2e` until you have a running frontend and set `PLAYWRIGHT_BASE_URL` to point at it. Until then, the E2E suite is prepared but will fail or be skipped. When you do have a frontend: (1) Start the app, (2) set `PLAYWRIGHT_BASE_URL`, (3) run `npm run test:e2e` from this folder (or `npm run test:e2e --prefix e2e` from repo root).

**Run from this folder:** `npm run test:e2e`

**Run from repo root:** `npm run test:e2e --prefix e2e`

Set `PLAYWRIGHT_BASE_URL` if your app runs on another port (e.g. `PLAYWRIGHT_BASE_URL=http://localhost:5173 npm run test:e2e`). Default is `http://localhost:3000`.

See [docs/setup/AUDIT_GATE.md](../docs/setup/AUDIT_GATE.md) for when the Audit Gate applies and how the agent uses this suite.
