---
name: "express-nodejs-web-security-spec (Part 2)"
description: "Security spec for Express 5.x/4.19.2+ Node.js apps covering secure code generation and vulnerability auditing. - Part 2"
---


## 5) Practical scanning heuristics (how to “hunt”)

When actively scanning an Express repo, these patterns are high-signal:

* TLS / transport:

  * `app.listen(80` without reverse proxy mention; missing `helmet`; cookies missing `secure` ([Express][1]) (NOTE this only applies to web facing applications, internal apps likely won't have TLS)
* Proxy trust:

  * `app.set('trust proxy', true)`; logic using `req.ip`/`req.protocol`/`req.hostname` ([Express][2])
* Security headers / fingerprinting:

  * missing `helmet(`; missing `app.disable('x-powered-by')` ([Express][1])
* Cookies / sessions:

  * `express-session` with missing `store` (MemoryStore risk), hard-coded `secret:`, missing `cookie: { secure/httpOnly/sameSite }` ([Express][1])
  * `cookie-session` storing large objects or secrets ([Express][1])
* Body parsing limits:

  * `express.json()` or `express.urlencoded()` without `limit`/`parameterLimit`/`depth` ([Express][5])
* CSRF:

  * POST/PUT/PATCH/DELETE routes using cookie auth with no CSRF tokens/origin checks ([OWASP Cheat Sheet Series][3])
* Open redirects:

  * `res.redirect(req.query.next)` or similar ([Express][1])
* XSS / HTML output:

  * `res.send(` building HTML with user input; template “safe” flags; untrusted values in `res.locals` ([Express][5])
* File handling:

  * `res.sendFile(` / `res.download(` where path originates from request; `express.static('uploads')` ([Express][5])
* Injection:

  * SQL strings + template literals into DB calls ([OWASP Cheat Sheet Series][6])
  * `child_process.exec` / `execSync` / `shell: true` ([OWASP Cheat Sheet Series][14])
* SSRF:

  * outbound `fetch/axios/got` to user-provided URLs ([OWASP Cheat Sheet Series][7])
* Brute force / abuse:

  * auth endpoints lacking throttling; no rate limiting middleware ([Express][1])
* Supply chain:

  * outdated Express versions; no lockfiles; no `npm audit` workflow ([Express][1])
* Node runtime hazards:

  * `--inspect` in production scripts; `insecureHTTPParser` usage ([Node.js][15])

Always try to confirm:

* data origin (untrusted vs trusted)
* sink type (HTML/template, SQL/NoSQL, subprocess, filesystem, redirect, outbound HTTP)
* protective controls present (validation, allowlists, middleware, proxy config, header policies)
* whether protections are at the edge vs in app code

---

## 6) Sources (accessed 2026-01-27)

Primary Express documentation:

* Express: Production Best Practices — Security: `https://expressjs.com/en/advanced/best-practice-security.html` ([Express][1])
* Express: Behind Proxies (`trust proxy`): `https://expressjs.com/en/guide/behind-proxies.html` ([Express][2])
* Express 5.x API Reference (parsers, static, sendFile, redirect, cookies): `https://expressjs.com/en/5x/api.html` ([Express][5])
* Express: Error Handling: `https://expressjs.com/en/guide/error-handling.html` ([Express][11])

Session middleware documentation:

* express-session docs (cookie flags, secret rotation, fixation mitigation, MemoryStore warning): `https://expressjs.com/en/resources/middleware/session.html` ([Express][1])

Node.js and npm official references:

* Node.js — Security Best Practices (DoS, proxy guidance, inspector risks, request smuggling notes): `https://nodejs.org/en/learn/getting-started/security-best-practices` ([Node.js][15])
* npm Docs — `npm audit`: `https://docs.npmjs.com/cli/v9/commands/npm-audit/` ([npm Docs][16])

OWASP Cheat Sheet Series:

* Session Management: `https://cheatsheetseries.owasp.org/cheatsheets/Session_Management_Cheat_Sheet.html` ([OWASP Cheat Sheet Series][12])
* CSRF Prevention: `https://cheatsheetseries.owasp.org/cheatsheets/Cross-Site_Request_Forgery_Prevention_Cheat_Sheet.html` ([OWASP Cheat Sheet Series][3])
* XSS Prevention: `https://cheatsheetseries.owasp.org/cheatsheets/Cross_Site_Scripting_Prevention_Cheat_Sheet.html` ([OWASP Cheat Sheet Series][4])
* Input Validation: `https://cheatsheetseries.owasp.org/cheatsheets/Input_Validation_Cheat_Sheet.html` ([OWASP Cheat Sheet Series][17])
* SQL Injection Prevention: `https://cheatsheetseries.owasp.org/cheatsheets/SQL_Injection_Prevention_Cheat_Sheet.html` ([OWASP Cheat Sheet Series][6])
* OS Command Injection Defense: `https://cheatsheetseries.owasp.org/cheatsheets/OS_Command_Injection_Defense_Cheat_Sheet.html` ([OWASP Cheat Sheet Series][14])
* SSRF Prevention: `https://cheatsheetseries.owasp.org/cheatsheets/Server_Side_Request_Forgery_Prevention_Cheat_Sheet.html` ([OWASP Cheat Sheet Series][7])
* File Upload: `https://cheatsheetseries.owasp.org/cheatsheets/File_Upload_Cheat_Sheet.html` ([OWASP Cheat Sheet Series][13])
* Unvalidated Redirects: `https://cheatsheetseries.owasp.org/cheatsheets/Unvalidated_Redirects_and_Forwards_Cheat_Sheet.html` ([OWASP Cheat Sheet Series][18])
* HTTP Headers: `https://cheatsheetseries.owasp.org/cheatsheets/HTTP_Headers_Cheat_Sheet.html` ([OWASP Cheat Sheet Series][10])

Versioning / advisories:

* Express package version (npm): `https://www.npmjs.com/package/express`
* Express open redirect advisory (CVE): `https://nvd.nist.gov/vuln/detail/CVE-2024-29041` ([NVD][9])

[1]: https://expressjs.com/en/advanced/best-practice-security.html "Security Best Practices for Express in Production"
[2]: https://expressjs.com/en/guide/behind-proxies.html "Express behind proxies"
[3]: https://cheatsheetseries.owasp.org/cheatsheets/Cross-Site_Request_Forgery_Prevention_Cheat_Sheet.html "Cross-Site Request Forgery Prevention - OWASP Cheat Sheet Series"
[4]: https://cheatsheetseries.owasp.org/cheatsheets/Cross_Site_Scripting_Prevention_Cheat_Sheet.html "Cross Site Scripting Prevention - OWASP Cheat Sheet Series"
[5]: https://expressjs.com/en/5x/api.html "Express 5.x - API Reference"
[6]: https://cheatsheetseries.owasp.org/cheatsheets/SQL_Injection_Prevention_Cheat_Sheet.html "SQL Injection Prevention - OWASP Cheat Sheet Series"
[7]: https://cheatsheetseries.owasp.org/cheatsheets/Server_Side_Request_Forgery_Prevention_Cheat_Sheet.html "Server Side Request Forgery Prevention - OWASP Cheat Sheet Series"
[8]: https://cheatsheetseries.owasp.org/cheatsheets/Nodejs_Security_Cheat_Sheet.html "Nodejs Security - OWASP Cheat Sheet Series"
[9]: https://nvd.nist.gov/vuln/detail/cve-2024-29041?utm_source=chatgpt.com "CVE-2024-29041 Detail - NVD"
[10]: https://cheatsheetseries.owasp.org/cheatsheets/HTTP_Headers_Cheat_Sheet.html "HTTP Headers - OWASP Cheat Sheet Series"
[11]: https://expressjs.com/en/guide/error-handling.html "Express error handling"
[12]: https://cheatsheetseries.owasp.org/cheatsheets/Session_Management_Cheat_Sheet.html "Session Management - OWASP Cheat Sheet Series"
[13]: https://cheatsheetseries.owasp.org/cheatsheets/File_Upload_Cheat_Sheet.html "File Upload - OWASP Cheat Sheet Series"
[14]: https://cheatsheet

<!-- Truncated for size -->
---

## Advanced Rules


### EXPRESS-XSS-001: Prevent reflected/stored XSS in HTML responses and templating

Severity: High

Required:

* MUST escape untrusted content in HTML output (templates should auto-escape by default; do not bypass).
* MUST NOT inject untrusted strings into HTML without escaping/sanitization.
* SHOULD set CSP (via Helmet) for apps rendering user-controlled content.
* SHOULD keep `res.locals` free of user-controlled input intended for templates unless it is validated/escaped.

Insecure patterns:

* `res.send("<div>" + req.query.q + "</div>")`
* Passing untrusted HTML through “safe” template flags/filters.
* Writing untrusted strings into `res.locals` and then rendering without escaping.

Detection hints:

* Search for `res.send(` with strings containing user input.
* Search for template “safe” flags (engine-specific) and trace data origin.
* Search for assignments to `res.locals` and whether they might contain untrusted data.

Fix:

* Use a template engine with autoescaping; pass only validated data.
* For rich text that must contain HTML, use a trusted sanitizer and an allowlist policy.
* Add CSP with realistic directives.

Notes:

* Express API docs explicitly warn that `res.locals` “should not contain user-controlled input” and is often used to expose things like CSRF tokens to templates. ([Express][5])
* OWASP XSS prevention guidance provides standard output-encoding and policy recommendations. ([OWASP Cheat Sheet Series][4])
* Helmet can mitigate some XSS classes via headers such as CSP. ([Express][1])

---

### EXPRESS-TEMPLATE-001: Never render untrusted templates or template paths (SSTI / LFI risk)

Severity: Critical (if you can prove template strings/paths are user/attacker-controlled)

Required:

* MUST NOT render templates whose contents or template path/name is influenced by untrusted input.
* MUST NOT load templates from user-controlled filesystem locations.
* SHOULD treat “email template editors”, “theme engines”, and “CMS-like template storage” as high-risk designs requiring sandboxing and isolation.

Insecure patterns:

* `res.render(req.query.view, data)` where `view` is not allowlisted.
* Rendering a template from a string that includes user input (engine-specific).
* Loading templates from uploads directories.

Detection hints:

* Search for `res.render(` where the first argument is derived from request/DB without allowlist.
* Search for template compilation APIs (engine-specific) fed by user content.

Fix:

* Use allowlisted template names and a fixed templates directory.
* If user-defined templates are required, implement strict sandboxing and isolate execution.

Notes:

* Express’s template system depends on the chosen engine; assume unsafe if user input influences template selection or source.

---

### EXPRESS-FILES-001: Prevent path traversal and unsafe file serving (sendFile/download)

Severity: High

Required:

* MUST NOT pass user-controlled filesystem paths directly to `res.sendFile()` / `res.download()` / filesystem APIs.
* SHOULD use `res.sendFile` with a fixed `root` and strict options (e.g., deny dotfiles) when serving user-selected files from a directory.
* MUST enforce authorization checks before serving user-specific files.

Insecure patterns:

* `res.sendFile(req.query.path)` or `res.download(req.params.file)` with no root restriction.
* File-serving routes that accept `..` segments, encoded traversal, or absolute paths.

Detection hints:

* Search for `res.sendFile(` and trace the `path` argument origin.
* Search for `res.download(` and trace the `path` argument origin.
* Look for `fs.readFile`/`createReadStream` on paths derived from requests.

Fix:

* Use an identifier-to-path mapping stored server-side (DB), not raw paths from clients.
* Use `root: <trusted_base_dir>` and `dotfiles: 'deny'` where appropriate; validate the filename component strictly.

Notes:

* Express’s `res.sendFile` docs show using a `root` option and `dotfiles: 'deny'` as part of a safe serving configuration. ([Express][5])
* `res.download` transfers the file as an attachment, but you still must control/validate the underlying `path`. ([Express][5])

---

### EXPRESS-STATIC-001: Harden `express.static` / serve-static and never serve untrusted uploads as active content

Severity: Medium (if serving untrusted user files if there are not robust limits tot eh file extensions)

Required:

* MUST NOT serve user uploads from a public static directory as active content (especially HTML/JS/SVG) unless explicitly intended and sandboxed. If sure that the content is inactive (png, jpg, other images etc) then it may be safe. It may be good to validate image file extensions are allow-listed before serving them.
* SHOULD configure static serving to:

  * deny/ignore dotfiles
  * avoid unintended directory indexes if not needed
  * apply appropriate cache controls for immutable assets

Insecure patterns:

* `app.use(express.static('uploads'))` where users can upload arbitrary files.
* Serving uploaded HTML or SVG inline from the same origin as the app.

Detection hints:

* Search for `express.static(` and identify served directories.
* Compare served directories with upload storage locations.
* Check for `dotfiles` and `index` options in static middleware.

Fix:

* Store uploads outside any static web root and serve via controlled routes that set safe `Content-Type` and `Content-Disposition: attachment` when appropriate.
* Configure `express.static(root, { dotfiles: 'deny'|'ignore', index: false (if desired) })`.

Notes:

* Express documents `express.static` options, including `dotfiles` behavior and `index`. ([Express][5])

---

### EXPRESS-UPLOAD-001: File uploads must be validated, stored safely, and served safely

Severity: Low - Medium

Required:

* SHOULD enforce upload size limits (app + edge).
* MUST validate file type using allowlists and content checks (not only filename extension).
* MUST store uploads outside executable/static roots when possible.
* SHOULD generate server-side filenames (random IDs); do not trust original names.
* MUST serve potentially active formats safely (download attachment) unless explicitly intended.

Insecure patterns:

* Accepting arbitrary file types and serving them back inline.
* Using `file.originalname` as the storage path.
* Missing size/type validation.

Detection hints:

* Look for multer/busboy/formidable usage and check for `limits`.
* Check where uploaded files are written and how they are served.
* Check whether uploads end up under `public/` or any `express.static` root.

Fix:

* Implement allowlist validation + safe storage + safe serving, per OWASP upload guidance.

Notes:

* OWASP File Upload guidance covers allowlists, content validation, storage, and safe serving patterns. ([OWASP Cheat Sheet Series][13])

---

### EXPRESS-INJECT-001: Prevent SQL injection (use parameterized queries / ORM)

Severity: High

Required:

* MUST use parameterized queries or an ORM/query builder that parameterizes under the hood.
* MUST NOT build SQL via string concatenation/template literals with untrusted input.

Insecure patterns:

* ``db.query(`SELECT * FROM users WHERE id = ${req.query.id}`)``
* `"SELECT ... WHERE name = '" + req.body.name + "'"`

Detection hints:

* Grep for `SELECT`, `INSERT`, `UPDATE`, `DELETE` strings in JS/TS.
* Trace untrusted input into `.query(...)`, `.execute(...)`, or raw SQL APIs.

Fix:

* Replace with parameterized queries (placeholders) or ORM query APIs.
* Validate types (e.g., integer IDs) before querying.

Notes:

* OWASP SQL injection prevention guidance strongly favors parameterized queries. ([OWASP Cheat Sheet Series][6])

---

### EXPRESS-INJECT-002: Prevent NoSQL injection / operator injection (Mongo-style)

Severity: High (app-dependent)

Required:

* MUST validate types and schemas for any query object built from untrusted input.
* MUST prevent operator injection (e.g., `$ne`, `$gt`, `$where`) if user input is merged into query objects.
* SHOULD consider defensive libraries/middleware when appropriate.

Insecure patterns:

* `collection.find(req.body)` where the body is attacker-controlled.
* Merging `req.query`/`req.body` into Mongo queries without schema validation.

Detection hints:

* Search for `find(`, `findOne(`, `aggregate(` calls where argument is request-derived.
* Check for patterns like `{ ...req.query }` or `Object.assign(query, req.body)`.

Fix:

* Use schema validation at boundary; explicitly construct query objects from validated fields only.

Notes:

* OWASP Node.js cheat sheet discusses input validation and mentions Node ecosystem modules commonly used for sanitization in NoSQL contexts. ([OWASP Cheat Sheet Series][8])

---

### EXPRESS-CMD-001: Prevent OS command injection (child_process)

Severity: Critical to High (depends on exposure), please prove it is user/attacker controlled

Required:

* MUST avoid executing shell commands with untrusted input.
* If subprocess is necessary:

  * MUST avoid `exec()` / `execSync()` with attacker-influenced strings
  * MUST NOT use `shell: true` with attacker-influenced data
  * SHOULD use `spawn()` with an argument array and strict allowlists. Ensure the executable is hardcoded or allow-listed, do not use a user supplied command name.
  * SHOULD place user-controlled values after `--` when supported by the subcommand to avoid flag injection

Insecure patterns:

* `exec(req.query.cmd)`
* `exec(`convert ${userPath} ...`)`
* `spawn('sh', ['-c', userString])`
* `spawn(userString, ['foo'])`

Detection hints:

* Search for `child_process`, `exec(`, `execSync(`, `spawn(`, `fork(`.
* Trace request/DB data into command construction.

Fix:

* If possible, write the functionality in javascript or use a library instead of subprocess.
* If unavoidable, hard-code command and strictly allowlist parameters.

Notes:

* OWASP OS command injection defense guidance covers avoid-shell and allowlist patterns. ([OWASP Cheat Sheet Series][14])

---

### EXPRESS-SSRF-001: Prevent server-side request forgery (SSRF) in outbound HTTP

Severity: Medium (High in cloud/LAN deployments)

NOTE: This is mostly only applicable to apps which will be deployed in a cloud/LAN setup or have other http services on the same box. Sometimes the feature requires this functionality unavoidably (webhooks).

Required:

* MUST treat outbound requests to user-provided URLs as high risk if there are other reachable private http endpoints.
* SHOULD validate and restrict destinations (allowlist hosts/domains) for any user-influenced URL fetch.
* SHOULD block access to:

  * localhost / private IP ranges / link-local
  * cloud metadata endpoints
* MUST allow only `http`/`https` for URL fetch features (to avoid schemas such as `file:`,`javascript:`)
* SHOULD set timeouts and restrict redirects.

Insecure patterns:

* `fetch(req.query.url)`
* “URL preview” / “import from URL” endpoints that accept arbitrary URLs.

Detection hints:

* Search for `fetch(`, `axios(`, `got(`, `request(`, `node-fetch` usage where URL originates from users/DB.
* Review webhook testers, previewers, image fetchers.

Fix:

* Enforce scheme allowlist, host allowlist, DNS/IP resolution checks, timeouts, and redirect policy.
* Consider network egress controls at infrastructure level.

Notes:

* OWASP SSRF prevention guidance provides standard controls and common pitfalls. ([OWASP Cheat Sheet Series][7])

---

### EXPRESS-ERROR-001: Error handling MUST not leak sensitive details in production

Severity: Low

Required:

* SHOULD define a centralized error handler (`app.use((err, req, res, next) => ...)`) at the end of middleware.
* MUST avoid returning stack traces, internal error messages, or secrets to clients in production.
* SHOULD log errors server-side with appropriate redaction.
* SHOULD ensure the app runs with production settings so default behavior doesn’t leak details.
* MUST avoid logging or returning sensitive information such as secrets, env vars, sessions, cookies in error messages in production.

Insecure patterns:

* Returning `err.stack` to clients.
* Using dev-only error middleware in production.
* `NODE_ENV` left as development, causing verbose error responses.

Detection hints:

* Verify there is a final error-handling middleware.
* Search for `res.status(500).send(err)` or similar.
* Check production environment variables and startup scripts.

Fix:

* Add a production-safe error handler that returns generic messages and logs details internally.
* Ensure environment is configured for production behavior.

Notes:

* Express production security guidance recommends custom error handling. ([Express][1])
* Express error handling docs describe the default error handler behavior and how production mode affects what is exposed. ([Express][11])

---

### EXPRESS-AUTH-001: Prevent brute-force attacks against authorization endpoints

Severity: Medium

NOTE: This is highly application specific and while it is good to bring to the attention of the user, it is hard to fix without additional complex configurations. Prefer to inform the user and if they request you to help implement a solution, help walk them through possible solutions.

Required:

* SHOULD protect login/auth endpoints against brute forcing.
* SHOULD rate-limit by:

  1. consecutive failed attempts per username+IP
  2. failed attempts per IP over a time window

Insecure patterns:

* Unlimited login attempts.

Detection hints:

* Identify all auth endpoints and check for rate limiting/throttling.
* Search for `rate-limiter-flexible`, `express-rate-limit`, or gateway policies.

Fix:

* Implement rate-limiting/throttling (app or edge). Express docs point to `rate-limiter-flexible` as a tool for this approach. ([Express][1])

Notes:

* OWASP Node.js cheat sheet also recommends precautions against brute forcing. ([OWASP Cheat Sheet Series][8])

---

### EXPRESS-DEPS-001: Dependency and patch hygiene (Express + Node + critical middleware)

Severity: Medium / Low

NOTE: `npm audit` often returns a large number of insignificant "vulnerabilities" which do not actually matter. You should only focus on Express or other extremely critical packages, ignoring ones listed in dev tools, bundlers, etc.

Do not upgrade packages without concent from the user. This may break existing code in unexpected ways. Instead, inform them of the outdated packages.

Required:

* MUST keep Express on a maintained version line (avoid EOL major versions).
* MAY use `npm audit` in CI and during maintenance work.
* SHOULD pin dependencies via lockfiles and review major updates carefully.

Insecure patterns:

* Running EOL Express versions (e.g., very old major lines).
* Ignoring `npm audit` findings without triage.
* Unpinned dependency ranges that auto-upgrade into insecure versions.

Detection hints:

* Check `package.json` and lockfiles for `express` version and other critical middleware versions.
* Inspect CI pipelines for `npm audit`/SCA steps.

Fix:

* Upgrade to latest stable Express and apply patches.
* Add automated dependency scanning and upgrade process.

Notes:

* Express production security guidance emphasizes that dependency vulnerabilities can compromise the app, and recommends `npm audit`. ([Express][1])
* Track security issues affecting Express versions (including known open-redirect-related CVEs). ([NVD][9])

---

### EXPRESS-DOS-001: Configure DoS protections (timeouts, limits, reverse proxy)

Severity: Low

NOTE: It may be hard to tell from the provided application context if the application runs behind a reverse proxy. You can inform the user or recommend one, but do not attempt to configure one without them initiating it. This is highly deployment dependant.

Required:

* SHOULD use a reverse proxy to provide caching, load balancing, and filtering controls when feasible.
* MAY configure server/proxy timeouts and connection limits to reduce exposure to Slowloris and similar DoS patterns.
* MUST ensure server/socket errors are handled so malformed connections do not crash the process. (Express should handle exceptions, but there are edgecases)

Insecure patterns:

* No reverse proxy in front of a public Node server, with defaults everywhere.
* Missing error handlers on server/socket objects.
* Extremely permissive timeouts and unlimited body sizes.

Detection hints:

* Inspect server creation (`http.createServer`, `https.createServer`) and whether timeouts are set.
* Check proxy/gateway config for timeouts and max body size.

Fix:

* Explain how to configure reverse proxy and timeouts, set request size limits
* add robust error handling middleware

Notes:

* Node’s security guidance for HTTP DoS discusses using reverse proxies and correctly configuring server timeouts. ([Node.js][15])

---

### EXPRESS-NODE-INSPECT-001: Do not expose the Node inspector in production

Severity: Critical

NOTE: Ensure that this detection is actually in the production path, and not just being used for local debugging.

Required:

* MUST NOT run Node with `--inspect` (especially bound to non-loopback) in production.
* MUST ensure `NODE_OPTIONS` or startup scripts do not enable inspector in prod.
* SHOULD firewall/debug locally only.

Insecure patterns:

* `node --inspect=0.0.0.0:9229 app.js` in production.
* Container/PM2/systemd configs enabling inspector.

Detection hints:

* Search for `--inspect` in Dockerfiles, Procfiles, systemd units, PM2 configs, npm scripts.
* Check `NODE_OPTIONS`.

Fix:

* Remove inspector flags from production start commands; restrict to local dev.

Notes:

* Node security guidance discusses inspector exposure risks (e.g., DNS rebinding) and recommends not running inspector in production. ([Node.js][15])

---

### EXPRESS-NODE-HTTP-001: Do not enable insecure HTTP parsing in production

Severity: High

NOTE: Ensure that this detection is actually in the production path, and not just being used for local dev.

Required:

* MUST NOT use Node’s `insecureHTTPParser` in production.
* MAY suggest configuring front-end proxies to normalize ambiguous requests to reduce request smuggling risk.

Insecure patterns:

* Creating an HTTP server with `{ insecureHTTPParser: true }`.

Detection hints:

* Search for `insecureHTTPParser` in server creation code.

Fix:

* Remove insecure parsing; rely on spec-compliant parsing and normalize at the edge.

Notes:

* Node security guidance explicitly recommends not using `insecureHTTPParser`. ([Node.js][15])

---

## 5) Practical scanning heuristics (how to “hunt”)

When actively scanning an Express repo, these patterns are high-signal:

* TLS / transport:

  * `app.listen(80` without reverse proxy mention; missing `helmet`; cookies missing `secure` ([Express][1]) (NOTE this only applies to web facing applications, internal apps likely won't have TLS)
* Proxy trust:

  * `app.set('trust proxy', true)`; logic using `req.ip`/`req.protocol`/`req.hostname` ([Express][2])
* Security headers / fingerprinting:

  * missing `helmet(`; missing `app.disable('x-powered-by')` ([Express][1])
* Cookies / sessions:

  * `express-session` with missing `store` (MemoryStore risk), hard-coded `secret:`, missing `cookie: { secure/httpOnly/sameSite }` ([Express][1])
  * `cookie-session` storing large objects or secrets ([Express][1])
* Body parsing limits:

  * `express.json()` or `express.urlencoded()` without `limit`/`parameterLimit`/`depth` ([Express][5])
* CSRF:

  * POST/PUT/PATCH/DELETE routes using cookie auth with no CSRF tokens/origin checks ([OWASP Cheat Sheet Series][3])
* Open redirects:

  * `res.redirect(req.query.next)` or similar ([Express][1])
* XSS / HTML output:

  * `res.send(` building HTML with user input; template “safe” flags; untrusted values in `res.locals` ([Express][5])
* File handling:

  * `res.sendFile(` / `res.download(` where path originates from request; `express.static('uploads')` ([Express][5])
* Injection:

  * SQL strings + template literals into DB calls ([OWASP Cheat Sheet Series][6])
  * `child_process.exec` / `execSync` / `shell: true` ([OWASP Cheat Sheet Series][14])
* SSRF:

  * outbound `fetch/axios/got` to user-provided URLs ([OWASP Cheat Sheet Series][7])
* Brute force / abuse:

  * auth endpoints lacking throttling; no rate limiting middleware ([Express][1])
* Supply chain:

  * outdated Express versions; no lockfiles; no `npm audit` workflow ([Express][1])
* Node runtime hazards:

  * `--inspect` in production scripts; `insecureHTTPParser` usage ([Node.js][15])

Always try to confirm:

* data origin (untrusted vs trusted)
* sink type (HTML/template, SQL/NoSQL, subprocess, filesystem, redirect, outbound HTTP)
* protective controls present (validation, allowlists, middleware, proxy config, header policies)
* whether protections are at the edge vs in app code

---

## 6) Sources (accessed 2026-01-27)

Primary Express documentation:

* Express: Production Best Practices — Security: `https://expressjs.com/en/advanced/best-practice-security.html` ([Express][1])
* Express: Behind Proxies (`trust proxy`): `https://expressjs.com/en/guide/behind-proxies.html` ([Express][2])
* Express 5.x API Reference (parsers, static, sendFile, redirect, cookies): `https://expressjs.com/en/5x/api.html` ([Express][5])
* Express: Error Handling: `https://expressjs.com/en/guide/error-handling.html` ([Express][11])

Session middleware documentation:

* express-session docs (cookie flags, secret rotation, fixation mitigation, MemoryStore warning): `https://expressjs.com/en/resources/middleware/session.html` ([Express][1])

Node.js and npm official references:

* Node.js — Security Best Practices (DoS, proxy guidance, inspector risks, request smuggling notes): `https://nodejs.org/en/learn/getting-started/security-best-practices` ([Node.js][15])
* npm Docs — `npm audit`: `https://docs.npmjs.com/cli/v9/commands/npm-audit/` ([npm Docs][16])

OWASP Cheat Sheet Series:

* Session Management: `https://cheatsheetseries.owasp.org/cheatsheets/Session_Management_Cheat_Sheet.html` ([OWASP Cheat Sheet Series][12])
* CSRF Prevention: `https://cheatsheetseries.owasp.org/cheatsheets/Cross-Site_Request_Forgery_Prevention_Cheat_Sheet.html` ([OWASP Cheat Sheet Series][3])
* XSS Prevention: `https://cheatsheetseries.owasp.org/cheatsheets/Cross_Site_Scripting_Prevention_Cheat_Sheet.html` ([OWASP Cheat Sheet Series][4])
* Input Validation: `https://cheatsheetseries.owasp.org/cheatsheets/Input_Validation_Cheat_Sheet.html` ([OWASP Cheat Sheet Series][17])
* SQL Injection Prevention: `https://cheatsheetseries.owasp.org/cheatsheets/SQL_Injection_Prevention_Cheat_Sheet.html` ([OWASP Cheat Sheet Series][6])
* OS Command Injection Defense: `https://cheatsheetseries.owasp.org/cheatsheets/OS_Command_Injection_Defense_Cheat_Sheet.html` ([OWASP Cheat Sheet Series][14])
* SSRF Prevention: `https://cheatsheetseries.owasp.org/cheatsheets/Server_Side_Request_Forgery_Prevention_Cheat_Sheet.html` ([OWASP Cheat Sheet Series][7])
* File Upload: `https://cheatsheetseries.owasp.org/cheatsheets/File_Upload_Cheat_Sheet.html` ([OWASP Cheat Sheet Series][13])
* Unvalidated Redirects: `https://cheatsheetseries.owasp.org/cheatsheets/Unvalidated_Redirects_and_Forwards_Cheat_Sheet.html` ([OWASP Cheat Sheet Series][18])
* HTTP Headers: `https://cheatsheetseries.owasp.org/cheatsheets/HTTP_Headers_Cheat_Sheet.html` ([OWASP Cheat Sheet Series][10])

Versioning / advisories:

* Express package version (npm): `https://www.npmjs.com/package/express`
* Express open redirect advisory (CVE): `https://nvd.nist.gov/vuln/detail/CVE-2024-29041` ([NVD][9])

[1]: https://expressjs.com/en/advanced/best-practice-security.html "Security Best Practices for Express in Production"
[2]: https://expressjs.com/en/guide/behind-proxies.html "Express behind proxies"
[3]: https://cheatsheetseries.owasp.org/cheatsheets/Cross-Site_Request_Forgery_Prevention_Cheat_Sheet.html "Cross-Site Request Forgery Prevention - OWASP Cheat Sheet Series"
[4]: https://cheatsheetseries.owasp.org/cheatsheets/Cross_Site_Scripting_Prevention_Cheat_Sheet.html "Cross Site Scripting Prevention - OWASP Cheat Sheet Series"
[5]: https://expressjs.com/en/5x/api.html "Express 5.x - API Reference"
[6]: https://cheatsheetseries.owasp.org/cheatsheets/SQL_Injection_Prevention_Cheat_Sheet.html "SQL Injection Prevention - OWASP Cheat Sheet Series"
[7]: https://cheatsheetseries.owasp.org/cheatsheets/Server_Side_Request_Forgery_Prevention_Cheat_Sheet.html "Server Side Request Forgery Prevention - OWASP Cheat Sheet Series"
[8]: https://cheatsheetseries.owasp.org/cheatsheets/Nodejs_Security_Cheat_Sheet.html "Nodejs Security - OWASP Cheat Sheet Series"
[9]: https://nvd.nist.gov/vuln/detail/cve-2024-29041?utm_source=chatgpt.com "CVE-2024-29041 Detail - NVD"
[10]: https://cheatsheetseries.owasp.org/cheatsheets/HTTP_Headers_Cheat_Sheet.html "HTTP Headers - OWASP Cheat Sheet Series"
[11]: https://expressjs.com/en/guide/error-handling.html "Express error handling"
[12]: https://cheatsheetseries.owasp.org/cheatsheets/Session_Management_Cheat_Sheet.html "Session Management - OWASP Cheat Sheet Series"
[13]: https://cheatsheetseries.owasp.org/cheatsheets/File_Upload_Cheat_Sheet.html "File Upload - OWASP Cheat Sheet Series"
[14]: https://cheatsheetseries.owasp.org/cheatsheets/OS_Command_Injection_Defense_Cheat_Sheet.html "OS Command Injection Defense - OWASP Cheat Sheet Series"
[15]: https://nodejs.org/en/learn/getting-started/security-best-practices "Security Best Practices for Node.js"
[16]: https://docs.npmjs.com/cli/v9/commands/npm-audit/ "npm audit - npm Docs"
[17]: https://cheatsheetseries.owasp.org/cheatsheets/Input_Validation_Cheat_Sheet.html "Input Validation - OWASP Cheat Sheet Series"
[18]: https://cheatsheetseries.owasp.org/cheatsheets/Unvalidated_Redirects_and_Forwards_Cheat_Sheet.html "Unvalidated Redirects and Forwards - OWASP Cheat Sheet Series"
