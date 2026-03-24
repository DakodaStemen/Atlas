---
name: electron-tauri-desktop
description: Patterns, security practices, IPC models, and operational concerns for building production desktop applications with Electron or Tauri 2. Covers the decision matrix, process architecture, command/IPC design, auto-updates, code signing, and platform-specific behaviors.
domain: frontend
category: desktop
tags: [Electron, Tauri, desktop-app, IPC, security, cross-platform, Rust, preload, context-isolation, auto-updater]
triggers: ["electron", "tauri", "desktop app", "cross-platform desktop", "main process", "renderer process", "tauri command", "preload script", "context bridge", "ipcMain", "ipcRenderer", "auto updater", "code signing", "desktop IPC"]
---

# Electron & Tauri Desktop App Patterns

## Decision Matrix: Tauri 2 vs Electron

| Dimension | Electron | Tauri 2 |
| --- | --- | --- |
| Binary size | 80–120 MB (bundles Chromium + Node) | 2.5–10 MB (uses system WebView) |
| Memory at idle | 200–300 MB | 30–50 MB |
| Startup time | 1–4 seconds | < 500 ms |
| Backend language | Node.js (JavaScript) | Rust |
| WebView | Bundled Chromium (consistent) | System WebView (WKWebView / WebView2 / WebKitGTK) |
| Mobile target | No | Yes (iOS + Android in v2) |
| Ecosystem maturity | Very mature; powers VS Code, Discord, Slack | Growing fast (35% YoY as of 2025); fewer third-party plugins |
| Security default | Opt-in hardening required | Deny-by-default ACL; externally audited |
| Cross-platform consistency | Excellent (same Chromium everywhere) | Good, but WebView rendering differences exist |

**Choose Electron when:** you need maximum ecosystem reach, your team is Node-only, rendering consistency across OS is critical, or you are extending an existing Electron app.

**Choose Tauri when:** bundle size or memory matters (embedded, consumer laptops, tray utilities), you want Rust's safety guarantees in the backend, the app is security-sensitive, or you need mobile targets from the same codebase.

---

## Electron Architecture

### Process Model

Electron runs two process types that must communicate over IPC — they share no memory.

**Main process** (`main.js` / `main.ts`)

- Single Node.js process; owns the application lifecycle.
- Creates and manages `BrowserWindow` instances.
- Has full Node and Electron API access (`app`, `shell`, `dialog`, `Tray`, `Menu`, filesystem, native modules).
- Registers IPC handlers with `ipcMain.handle()` / `ipcMain.on()`.

**Renderer process** (one per window)

- Runs the web UI (Chromium). Has no Node access when properly configured.
- Communicates with main exclusively through a preload bridge.
- Multiple renderer processes are isolated from each other.

#### Preload script

- Runs in the renderer's process before page code loads, but in a separate context.
- Only place that may safely import Node/Electron APIs and re-expose a narrowed surface via `contextBridge`.

### Mandatory Security Configuration

Always set these `BrowserWindow` `webPreferences`:

```typescript
import { BrowserWindow } from 'electron';
import path from 'path';

const win = new BrowserWindow({
  webPreferences: {
    preload: path.join(__dirname, 'preload.js'),
    contextIsolation: true,   // isolates preload from renderer globals
    nodeIntegration: false,   // renderer cannot require() Node modules
    sandbox: true,            // OS-level process sandboxing (default since v20)
    webSecurity: true,        // never disable
  },
});
```

`contextIsolation: true` has been the default since Electron 12. Disabling it (or disabling `sandbox`) re-unifies JavaScript contexts and exposes Electron internals to the renderer — do not do this.

### Preload Script and contextBridge

Expose only what the renderer needs. Never expose raw `ipcRenderer`:

```typescript
// preload.ts
import { contextBridge, ipcRenderer } from 'electron';

contextBridge.exposeInMainWorld('electronAPI', {
  // Two-way: renderer calls, main responds
  openFile: () => ipcRenderer.invoke('dialog:openFile'),

  // One-way: renderer fires, main handles
  setTitle: (title: string) => ipcRenderer.send('window:setTitle', title),

  // Main-to-renderer: wrap the listener so _event is never exposed
  onUpdateAvailable: (callback: (version: string) => void) =>
    ipcRenderer.on('update:available', (_event, version) => callback(version)),
});
```

In the renderer (TypeScript declarations needed):

```typescript
// renderer.ts
const filePath = await window.electronAPI.openFile();
```

### IPC Patterns

#### Renderer → Main, two-way (preferred for data requests)

```typescript
// main.ts
import { ipcMain, dialog } from 'electron';

ipcMain.handle('dialog:openFile', async (event) => {
  // Validate sender before trusting
  if (!isTrustedSender(event.senderFrame)) return null;
  const { filePaths } = await dialog.showOpenDialog({});
  return filePaths[0] ?? null;
});

function isTrustedSender(frame: Electron.WebFrameMain): boolean {
  const url = new URL(frame.url);
  return url.protocol === 'file:' || url.hostname === 'localhost';
}
```

#### Main → Renderer (push notifications)

```typescript
// main.ts
win.webContents.send('update:available', '2.1.0');
```

**Renderer → Renderer**: no direct path exists. Route through main as a broker, or set up a `MessageChannel` / `MessagePort` pair via main after both windows have loaded.

**IPC data constraint**: only Structured Clone Algorithm-compatible values pass through IPC. DOM nodes, class instances with methods, and certain Node objects cannot be serialized — stick to plain objects, arrays, and primitives.

### Content Security Policy

Set a strict CSP. For local files use a meta tag; for served content use response headers:

```typescript
// main.ts — via session interceptor
import { session } from 'electron';

session.defaultSession.webRequest.onHeadersReceived((details, callback) => {
  callback({
    responseHeaders: {
      ...details.responseHeaders,
      'Content-Security-Policy': [
        "default-src 'none'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src 'self'"
      ],
    },
  });
});
```

Avoid `'unsafe-eval'` and `'unsafe-inline'` for scripts. Never use `*` as a source.

### Navigation and Window-Open Hardening

```typescript
// main.ts
win.webContents.on('will-navigate', (event, url) => {
  if (!url.startsWith('file://') && !url.startsWith('https://yourapp.com')) {
    event.preventDefault();
  }
});

win.webContents.setWindowOpenHandler(({ url }) => {
  if (isSafeExternal(url)) shell.openExternal(url);
  return { action: 'deny' };
});
```

### Electron Auto-Updater

Use `electron-updater` (from `electron-builder`) for cross-platform support:

```typescript
// main.ts
import { autoUpdater } from 'electron-updater';

autoUpdater.checkForUpdatesAndNotify();

autoUpdater.on('update-downloaded', () => {
  // Prompt user, then:
  autoUpdater.quitAndInstall();
});
```

#### Platform notes

- **macOS**: requires code-signed builds; notarization is required for Gatekeeper.
- **Windows**: Squirrel.Windows handles install; NSIS installer is also common via electron-builder.
- **Linux**: AppImage self-update works; Snap/deb/rpm require distribution channels.

For public GitHub-hosted apps, `update-electron-app` wraps `update.electronjs.org` as a zero-config solution.

### Electron Code Signing

**macOS**: requires Apple Developer certificate + notarization. Configure in `electron-builder`:

```json
// package.json (electron-builder config)
{
  "mac": {
    "identity": "Developer ID Application: Your Name (TEAMID)",
    "hardenedRuntime": true,
    "gatekeeperAssess": false,
    "entitlements": "build/entitlements.mac.plist",
    "entitlementsInherit": "build/entitlements.mac.plist",
    "notarize": { "teamId": "TEAMID" }
  }
}
```

**Windows**: requires EV or OV code-signing certificate. EV certs bypass SmartScreen immediately; OV certs build reputation over time.

---

## Tauri 2 Architecture

### Process Model (Tauri 2 Architecture)

Tauri has a **Core process** (Rust binary, equivalent to main) and one or more **WebView processes** (the frontend, equivalent to renderer). The WebView communicates with Core via the Tauri IPC bridge — exposed as `invoke()` in `@tauri-apps/api/core`.

### Defining Commands (Rust Backend)

```rust
// src-tauri/src/main.rs or a commands module

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
}

#[tauri::command]
async fn read_file(path: String, app: tauri::AppHandle) -> Result<String, String> {
    std::fs::read_to_string(&path).map_err(|e| e.to_string())
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![greet, read_file])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

Commands can inject `AppHandle`, `Window`, `State<T>`, and `Channel` (for streaming) as parameters by type — Tauri's DI resolves them automatically.

### Invoking Commands from Frontend

```typescript
import { invoke } from '@tauri-apps/api/core';

// Matches the Rust command name (snake_case → camelCase or explicit rename)
const greeting = await invoke<string>('greet', { name: 'world' });

// With error handling
try {
  const content = await invoke<string>('read_file', { path: '/tmp/data.txt' });
} catch (err) {
  console.error('Command failed:', err);
}
```

Plugin commands use a namespaced form: `invoke('plugin:name|command_name', args)`.

### Tauri v2 Permissions and Capabilities (ACL System)

Tauri 2 replaced the v1 allowlist with a three-layer ACL:

**Permissions** — declare what commands a plugin/app exposes and whether they're allowed or denied, with optional scopes:

```toml
# src-tauri/permissions/read-user-files.toml
[[permission]]
identifier = "read-user-files"
description = "Allow reading files under the user's home directory"
commands.allow = ["read_file"]

[[scope.allow]]
path = "$HOME/**"

[[scope.deny]]
path = "$HOME/.ssh/**"
```

Tauri auto-generates `allow-<command>` and `deny-<command>` permissions for every command.

**Capabilities** — bind permissions to specific windows or webviews:

```json
// src-tauri/capabilities/main-window.json
{
  "identifier": "main-window",
  "description": "Permissions for the main application window",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "read-user-files",
    "tauri-plugin-fs:read-files",
    "tauri-plugin-dialog:default"
  ]
}
```

**Key principle**: everything is denied by default. A command that has no permission entry granting it is not callable from the frontend, regardless of whether it is registered in `invoke_handler`.

### Tauri Plugins

First-party plugins cover most system needs: `tauri-plugin-fs`, `tauri-plugin-dialog`, `tauri-plugin-http`, `tauri-plugin-shell`, `tauri-plugin-notification`, `tauri-plugin-updater`, `tauri-plugin-store`, `tauri-plugin-sql`.

Add a plugin:

```toml
# src-tauri/Cargo.toml
[dependencies]
tauri-plugin-fs = "2"
```

```rust
// main.rs
tauri::Builder::default()
    .plugin(tauri_plugin_fs::init())
    .invoke_handler(tauri::generate_handler![...])
    .run(tauri::generate_context!())
    .unwrap();
```

Then grant its permissions in a capability file.

### Tauri v2 Events (Frontend ↔ Backend)

For push notifications from backend to frontend, emit events rather than polling:

```rust
// Rust: emit to all windows
app.emit("file-changed", payload)?;

// Rust: emit to a specific window
window.emit("file-changed", payload)?;
```

```typescript
// Frontend: listen
import { listen } from '@tauri-apps/api/event';

const unlisten = await listen<string>('file-changed', (event) => {
  console.log('changed:', event.payload);
});
// Call unlisten() when component unmounts
```

For streaming data from a long-running command (progress bars, log tails), use `Channel`:

```rust
#[tauri::command]
async fn long_task(on_progress: tauri::ipc::Channel<u8>) {
    for i in 0..=100u8 {
        on_progress.send(i).unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
}
```

```typescript
import { Channel, invoke } from '@tauri-apps/api/core';

const onProgress = new Channel<number>();
onProgress.onmessage = (pct) => setProgress(pct);
await invoke('long_task', { onProgress });
```

### Tauri Auto-Updater

Install the plugin:

```toml
[dependencies]
tauri-plugin-updater = "2"
```

```json
// tauri.conf.json
{
  "plugins": {
    "updater": {
      "pubkey": "YOUR_PUBLIC_KEY_HERE",
      "endpoints": [
        "https://releases.yourapp.com/{{target}}/{{arch}}/{{current_version}}"
      ],
      "createUpdaterArtifacts": true
    }
  }
}
```

Generate signing keys once:

```sh
tauri signer generate -w ~/.tauri/myapp.key
# Outputs public key to embed in config, keep private key offline/in CI secrets
```

Frontend update flow:

```typescript
import { check } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';

const update = await check();
if (update) {
  let downloaded = 0;
  await update.downloadAndInstall((event) => {
    if (event.event === 'Progress') downloaded = event.data.chunkLength;
    if (event.event === 'Finished') console.log('Download complete');
  });
  await relaunch();
}
```

**Platform note**: on Windows the app process exits during install; save all state before calling `downloadAndInstall`.

Signature verification is mandatory and cannot be disabled — every update artifact must be signed with the private key whose public counterpart is embedded in the config.

### Tauri Code Signing

Bundle via `tauri build`. Signing is configured in `tauri.conf.json` under the platform bundle section:

```json
{
  "bundle": {
    "macOS": {
      "signingIdentity": "Developer ID Application: ...",
      "providerShortName": "TEAMID",
      "entitlements": "entitlements.plist"
    },
    "windows": {
      "certificateThumbprint": "ABCDEF...",
      "timestampUrl": "http://timestamp.digicert.com"
    }
  }
}
```

The `tauri` CLI integrates with `cargo-packager` to produce signed `.dmg`/`.app` on macOS, NSIS/MSI on Windows, and AppImage/deb/rpm on Linux.

---

## Platform-Specific Behaviors

### Windows

- **Electron**: uses WinRT APIs via Node native modules or `node-windows`. Default renderer is Chromium so rendering is consistent.
- **Tauri**: uses WebView2 (Chromium-based, ships with Windows 11, auto-installed on 10). WebView2 version is tied to the OS update cycle — test minimum supported WebView2 version. NSIS or MSI installer; app restarts on update.
- Both: NSIS silent install flags differ from macOS DMG drag-install; sign with EV cert to avoid SmartScreen warnings on first run.

### macOS

- **Notarization is required** for both frameworks to pass Gatekeeper on macOS 10.15+. Apple Silicon requires universal binaries or separate arm64/x64 builds.
- Tauri's `tauri build --target universal-apple-darwin` produces a fat binary.
- Electron: set `hardenedRuntime: true` and include appropriate entitlements (e.g., `com.apple.security.cs.allow-jit` if needed for V8).
- Both need `NSLocalNetworkUsageDescription` and other `Info.plist` keys for any sensitive capability.

### Linux

- **Electron**: ships its own Chromium; consistent but heavyweight.
- **Tauri**: depends on `webkit2gtk-4.1`. Rendering differences between GTK/WebKit versions across distros are a real concern — test on Ubuntu LTS and Fedora at minimum.
- AppImage is the most portable Linux target (self-contained, no install). Flatpak/Snap are better for distribution store presence.

---

## Security Checklist

### Electron

- [ ] `contextIsolation: true`, `nodeIntegration: false`, `sandbox: true` on every `BrowserWindow`
- [ ] Preload uses `contextBridge` — never exposes raw `ipcRenderer` or `require`
- [ ] All `ipcMain.handle` / `ipcMain.on` handlers validate `event.senderFrame.url`
- [ ] CSP header set via session interceptor; no `unsafe-eval`
- [ ] `will-navigate` and `setWindowOpenHandler` guard all navigation
- [ ] `shell.openExternal` called only with validated `https://` URLs
- [ ] `allowRunningInsecureContent: false` (default)
- [ ] Electron version kept within supported range (last 3 stable releases get security patches)
- [ ] `@electron/fuses` used to disable unused runtime features at build time

### Tauri

- [ ] All frontend-accessible commands have explicit `allow-*` permissions granted in a capability
- [ ] Scopes restrict filesystem/URL access to the minimum required paths
- [ ] Capabilities are scoped to specific window identifiers, not `*`
- [ ] Updater public key is embedded in config; private key is stored in CI secrets only
- [ ] CSP set in `tauri.conf.json` under `security.csp`
- [ ] `dangerousDisableAssetCspModification` is not set
- [ ] Isolation pattern used if third-party iframes or untrusted content is loaded

---

## State Management and Native Integration

### Electron (State Management and Native Integration)

Manage main-process state in module-level singletons or a simple state object. Persist with `electron-store` (JSON file in `app.getPath('userData')`). For SQLite, use `better-sqlite3` (sync, simpler) or `sqlite3` (async).

```typescript
import Store from 'electron-store';
const store = new Store<{ theme: string }>();
store.set('theme', 'dark');
```

### Tauri (State Management and Native Integration)

Use `tauri-plugin-store` for lightweight key-value persistence:

```typescript
import { load } from '@tauri-apps/plugin-store';
const store = await load('settings.json', { autoSave: true });
await store.set('theme', 'dark');
```

For SQLite, `tauri-plugin-sql` exposes a Rust SQLx connection to the frontend via commands. Alternatively, manage the DB entirely in Rust and expose typed commands.

Manage shared mutable state in Rust with `tauri::State` and `Mutex`:

```rust
use std::sync::Mutex;

struct AppState {
    counter: Mutex<u32>,
}

#[tauri::command]
fn increment(state: tauri::State<'_, AppState>) -> u32 {
    let mut n = state.counter.lock().unwrap();
    *n += 1;
    *n
}

fn main() {
    tauri::Builder::default()
        .manage(AppState { counter: Mutex::new(0) })
        .invoke_handler(tauri::generate_handler![increment])
        .run(tauri::generate_context!())
        .unwrap();
}
```

---

## Tooling and Build

| Concern | Electron | Tauri |
| --- | --- | --- |
| Bundler | electron-builder or electron-forge | `tauri build` (wraps cargo + bundler) |
| Dev server | webpack/Vite + `electron .` | `tauri dev` (starts Vite/Webpack + Rust watch) |
| TypeScript | standard TS setup | standard TS for frontend; Rust for backend |
| Native modules | node-gyp, requires rebuild per Electron version | Rust crates; no binding rebuild step |
| CI matrix | separate builds per platform | same; cross-compilation to macOS/Windows from Linux is limited for native code |

For Electron with Vite, `electron-vite` (`@electron-toolkit/preload`, `@electron-toolkit/utils`) provides an opinionated monorepo structure with separate entry points for main, preload, and renderer.

For Tauri, the official `create-tauri-app` scaffolds the project with your preferred frontend framework (React, Svelte, Vue, etc.) and wires up `tauri dev` / `tauri build`.
