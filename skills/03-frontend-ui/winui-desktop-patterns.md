---
name: winui-desktop-patterns
description: Complete WinUI 3 and Windows App SDK reference. Covers environment setup, app bootstrap, project structure, controls, layout, adaptive UI, styling, theming, Mica/Acrylic materials, accessibility, localization, motion, build/run/launch verification, and troubleshooting. Use for any WinUI 3 desktop application development task.
domain: frontend
category: desktop
tags: [WinUI, WinUI3, Windows-App-SDK, XAML, CSharp, Fluent, desktop, Mica, Acrylic]
triggers: WinUI, WinUI 3, Windows App SDK, XAML, WinUI app, Fluent design, desktop app Windows
---

# WinUI Desktop Patterns

Unified reference for WinUI 3 and Windows App SDK development.

---

## 1. Environment Setup & Bootstrap

### Required Flow

1. Classify task: environment/setup, new-app bootstrap, design, implementation, review, or troubleshooting.
2. For fresh setup, run the bundled WinGet configuration:

```powershell
winget configure -f config.yaml --accept-configuration-agreements --disable-interactivity
```

3. Verify template availability:

```powershell
dotnet new list winui
```

4. Scaffold new app:

```powershell
dotnet new winui -o <AppName>
```

Supported options: `-f|--framework net10.0|net9.0|net8.0`, `-slnx|--use-slnx`, `-cpm|--central-pkg-mgmt`, `-mvvm|--use-mvvm`, `-imt|--include-mvvm-toolkit`, `-un|--unpackaged`, `-nsf|--no-solution-file`, `--force`.

5. Verify scaffold: confirm `.csproj` exists and `dotnet build` succeeds.
6. Launch and confirm real top-level window appears.

### Packaging Model Decision

- **Packaged (default):** Store-like workflows, Visual Studio deploy/F5. Package identity and package-backed storage.
- **Unpackaged:** CLI build-and-run loops, direct `.exe` launches. Must guard APIs requiring package identity (e.g., `ApplicationData.Current` can fail).

---

## 2. Project Structure

### Recommended Layout

```
App.xaml / App.xaml.cs     — global resources, startup, window creation
MainWindow.xaml/.cs        — shell, title bar, navigation host
Pages/                     — page views and page-specific logic
Controls/                  — reusable WinUI user controls
ViewModels/                — state and commands (when separation benefits the app)
Styles/                    — resource dictionaries, theme tokens, shared control styles
Helpers/ or Services/      — windowing, navigation, persistence, OS integration
```

### Binding Guidance

- Prefer `x:Bind` for page-local properties, event handlers, strongly typed VM access.
- Use `Binding` where data context is dynamic or templates must stay flexible.
- Avoid binding patterns depending on unclear page lifetime or implicit data contexts.

### Key Rules

- Centralize app resources in `App.xaml` and shared resource dictionaries.
- Separate shell logic from content pages.
- Do not introduce MVVM ceremony the project will not maintain.
- Use native `CommandBar` for grouped actions before custom toolbar compositions.

---

## 3. Controls, Layout & Adaptive UI

### Control Selection

| Need | Prefer |
| --- | --- |
| Forms/settings | Native controls first; Toolkit settings controls only if clearly beneficial |
| Command surfaces | `CommandBar` with overflow model before custom button rows |
| Large collections | Virtualization-friendly controls (`ListView`, `GridView`) |
| Horizontal shelves in vertical scroll | Horizontal `ScrollViewer` + `ItemsRepeater` (NOT nested `GridView`) |
| Search/filtering | Single search field with live updates (add explicit controls only for expensive ops) |
| Modal decisions | `ContentDialog` |
| Persistent status | `InfoBar` |
| Contextual onboarding | `TeachingTip` |

### Adaptive Layout Rules

- Design with effective pixels, not fixed device assumptions.
- Make the smallest supported layout fully usable.
- Plan explicit wide, medium, and phone-width behavior.
- Use visual states and adaptive triggers intentionally.
- When width shrinks: drop secondary controls or move behind shell affordances.
- Phone-width: stack items vertically instead of clipped horizontal rails.
- Make scroll ownership explicit: if page scrolls vertically, nested collections need their own explicit scroll handling.

### Anti-Patterns

- Do not replace standard controls with custom ones just for appearance.
- Do not hard-code sizes for a single window width.
- Do not wrap sections in extra `Border` when spacing/headers already establish grouping.
- Do not nest scroll-owning `GridView` inside outer `ScrollViewer` without deciding scroll ownership.

---

## 4. Styling, Theming, Materials & Icons

### Theme Support

- Support light, dark, and high-contrast by default.
- Centralize brushes, typography, and spacing in shared resource dictionaries.
- Use system resources: `CardBackgroundFillColorDefaultBrush`, `CardStrokeColorDefaultBrush`, `LayerFillColorDefaultBrush`.

### Materials

| Material | Use For |
| --- | --- |
| **Mica** | Long-lived surfaces (main window background, title bar region) |
| **Acrylic** | Transient surfaces (flyouts, menus, light-dismiss) |

Verify fallback behavior on older Windows versions.

### Typography & Icons

- Use Segoe UI Variable or platform-default typography.
- Use Fluent iconography matching platform language.
- Create hierarchy through typography, not extra borders/decoration.
- For metadata containers: prefer small rounded rectangles over bright oval pills.

### Anti-Patterns

- No hard-coded light-theme colors (breaks dark/high-contrast).
- No custom `Border` wrappers when standard WinUI surfaces suffice.
- No "card around cards" (outer `Border` wrapping child items that already render as cards).
- No mixing unrelated icon styles.

---

## 5. Motion, Animations & Polish

### Prefer

- Motion that clarifies hierarchy, continuity, and state changes.
- Theme transitions, connected animations, built-in platform behaviors first.
- Short, purposeful animations supporting the task.

### Avoid

- Decorative animation that delays interaction.
- Multiple overlapping animations for same state change.
- Animation that hides focus, selection, or accessibility state.

### Guidance

- Use connected animation for real source-to-destination relationships.
- CommunityToolkit animation helpers only when built-in transitions insufficient.

---

## 6. Accessibility, Input & Localization

### Requirements

- Accessible names, help text, and landmarks for meaningful UI elements.
- Full keyboard reachability for main workflow.
- High-contrast-safe visuals.
- Localizable strings and layouts tolerating growth.
- Equal support for mouse, touch, pen, and keyboard.

### Checklist

- [ ] Keyboard-only user can complete the task
- [ ] Narrator has enough information to describe important UI
- [ ] Experience stays legible in high contrast
- [ ] Strings and layout ready for localization and RTL growth

---

## 7. Build, Run & Launch Verification

### Required Steps

1. Identify build target (solution/project, configuration, platform, packaging model).
2. Build after each meaningful code edit and at task completion.
3. Run the app after changes. Always when startup/navigation/resources/packaging changed.
4. Launch path must match deployment model:
   - Packaged: Visual Studio deploy or package-aware flow
   - Unpackaged: built `.exe` from `bin\Debug\...\win-x64\`
5. Verify with objective evidence: non-zero window handle, expected title, responsive process, no startup exception.
6. Leave verified app running for user unless explicitly told not to.

### Debugging Startup Failures

- Separate environment problems from app-code crashes.
- If app exits before showing window, inspect: `App.xaml` -> merged dictionaries -> converters -> `MainWindow` -> startup services.
- For opaque `MSB3073` / `XamlCompiler.exe` failures: simplify back toward `dotnet new winui` scaffold.
- Restore complex pieces incrementally. Minimal `App.xaml` + minimal `MainWindow` is a valid isolation step.
- For unpackaged startup: review persistence/notifications/storage code for hidden package-identity assumptions.

### Exit Criteria

- [ ] Build succeeds from intended workflow
- [ ] App launches from intended workflow
- [ ] Real top-level window confirmed
- [ ] No unresolved startup exception

---

## 8. Reference Priorities

1. **Microsoft Learn** for requirements, API expectations, platform guidance.
2. **WinUI Gallery** for concrete control usage, shell composition, design details.
3. **WindowsAppSDK-Samples** for scenario-level APIs (windowing, lifecycle, notifications, deployment).
4. **CommunityToolkit** only when built-in WinUI controls do not cover the need cleanly.

### General Rules

- Keep C# as primary path. Mention C++/WinRT only when material.
- Preserve conventions of existing codebase over generic sample structure.
- Treat WinUI design guidance and native controls as baseline.
- Do not invent app-specific controls or bespoke component libraries to replace stock WinUI behavior unless explicitly requested.
