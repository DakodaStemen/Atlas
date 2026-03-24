---
name: dotnet-aspnet-winui-stack
description: Consolidated .NET stack - ASP.NET Core, Razor Pages, CommunityToolkit, Windows App SDK lifecycle and deployment.
domain: backend
triggers: aspnet core, razor pages, community toolkit, windows app sdk, winui, dotnet, csharp
---

# .NET / ASP.NET / WinUI Stack

Consolidated .NET development patterns: ASP.NET Core, Razor Pages, CommunityToolkit controls, and Windows App SDK lifecycle/deployment.


---

<!-- merged from: aspnet-core.md -->

﻿---
name: aspnet-core
description: Build, review, refactor, or architect ASP.NET Core web applications using current official guidance for .NET web development. Use when working on Blazor Web Apps, Razor Pages, MVC, Minimal APIs, controller-based Web APIs, SignalR, gRPC, middleware, dependency injection, configuration, authentication, authorization, testing, performance, deployment, or ASP.NET Core upgrades.
---

# ASP.NET Core

## Overview

Choose the right ASP.NET Core application model, compose the host and request pipeline correctly, and implement features in the framework style Microsoft documents today.

Load the smallest set of references that fits the task. Do not load every reference by default.

## Workflow

1. Confirm the target framework, SDK, and current app model.
2. Open [references/stack-selection.md](references/stack-selection.md) first for new apps or major refactors.
3. Open [references/program-and-pipeline.md](references/program-and-pipeline.md) next for `Program.cs`, DI, configuration, middleware, routing, logging, and static assets.
4. Open exactly one primary app-model reference:
   - [references/ui-blazor.md](references/ui-blazor.md)
   - [references/ui-razor-pages.md](references/ui-razor-pages.md)
   - [references/ui-mvc.md](references/ui-mvc.md)
   - [references/apis-minimal-and-controllers.md](references/apis-minimal-and-controllers.md)
5. Add cross-cutting references only as needed:
   - [references/data-state-and-services.md](references/data-state-and-services.md)
   - [references/security-and-identity.md](references/security-and-identity.md)
   - [references/realtime-grpc-and-background-work.md](references/realtime-grpc-and-background-work.md)
   - [references/testing-performance-and-operations.md](references/testing-performance-and-operations.md)
6. Open [references/versioning-and-upgrades.md](references/versioning-and-upgrades.md) before introducing new platform APIs into an older solution or when migrating between major versions.
7. Use [references/source-map.md](references/source-map.md) when you need the Microsoft Learn section that corresponds to a task not already covered by the focused references.

## Default Operating Assumptions

- Prefer the latest stable ASP.NET Core and .NET unless the repository or user request pins an older target.
- As of March 2026, prefer .NET 10 / ASP.NET Core 10 for new production work. Treat ASP.NET Core 11 as preview unless the user explicitly asks for preview features.
- Prefer `WebApplicationBuilder` and `WebApplication`. Avoid older `Startup` and `WebHost` patterns unless the codebase already uses them or the task is migration.
- Prefer built-in DI, options/configuration, logging, ProblemDetails, OpenAPI, health checks, rate limiting, output caching, and Identity before adding third-party infrastructure.
- Keep feature slices cohesive so the page, component, endpoint, controller, validation, service, data access, and tests are easy to trace.
- Respect the existing app model. Do not rewrite Razor Pages to MVC or controllers to Minimal APIs without a clear reason.

## Reference Guide

- [references/_sections.md](references/_sections.md): Quick index and reading order.
- [references/stack-selection.md](references/stack-selection.md): Choose the right ASP.NET Core application model and template.
- [references/program-and-pipeline.md](references/program-and-pipeline.md): Structure `Program.cs`, services, middleware, routing, configuration, logging, and static assets.
- [references/ui-blazor.md](references/ui-blazor.md): Build Blazor Web Apps, choose render modes, and use components, forms, and JS interop correctly.
- [references/ui-razor-pages.md](references/ui-razor-pages.md): Build page-focused server-rendered apps with handlers, model binding, and conventions.
- [references/ui-mvc.md](references/ui-mvc.md): Build controller/view applications with clear separation of concerns.
- [references/apis-minimal-and-controllers.md](references/apis-minimal-and-controllers.md): Build HTTP APIs with Minimal APIs or controllers, including validation and response patterns.
- [references/data-state-and-services.md](references/data-state-and-services.md): Use EF Core, `DbContext`, options, `IHttpClientFactory`, session, temp data, and app state responsibly.
- [references/security-and-identity.md](references/security-and-identity.md): Apply authentication, authorization, Identity, secrets, data protection, CORS, CSRF, and HTTPS guidance.
- [references/realtime-grpc-and-background-work.md](references/realtime-grpc-and-background-work.md): Use SignalR, gRPC, and hosted services.
- [references/testing-performance-and-operations.md](references/testing-performance-and-operations.md): Add integration tests, browser tests, caching, compression, health checks, rate limits, and deployment concerns.
- [references/versioning-and-upgrades.md](references/versioning-and-upgrades.md): Handle target frameworks, breaking changes, obsolete APIs, and migrations.
- [references/source-map.md](references/source-map.md): Map the official ASP.NET Core documentation tree to the references in this skill.

## Execution Notes

- When generating new code, start from the correct `dotnet new` template and keep the generated structure recognizable.
- When editing an existing solution, follow the solution's conventions first and use these references to avoid framework misuse or outdated patterns.
- When a task mentions "latest", verify the feature on Microsoft Learn or the ASP.NET Core docs repo before relying on memory.


---

<!-- merged from: aspnet-core-source-map.md -->

﻿---
name: ASP.NET Core Source Map
description: # ASP.NET Core Source Map
 
 This skill is synthesized from the official ASP.NET Core documentation tree and overview pages. Use this file to map a task to the corresponding Microsoft Learn area before opening deeper docs.
---

# ASP.NET Core Source Map

This skill is synthesized from the official ASP.NET Core documentation tree and overview pages. Use this file to map a task to the corresponding Microsoft Learn area before opening deeper docs.

Core sources:

- <https://learn.microsoft.com/aspnet/core/>
- <https://raw.githubusercontent.com/dotnet/AspNetCore.Docs/main/aspnetcore/toc.yml>
- <https://github.com/dotnet/AspNetCore.Docs/tree/main/aspnetcore>

## Documentation Tree Mapping

| ASP.NET Core docs area | Use this skill reference first |
| --- | --- |
| Overview, Get started, What's new | `stack-selection.md`, `versioning-and-upgrades.md` |
| Fundamentals | `program-and-pipeline.md` |
| Web apps | `ui-blazor.md`, `ui-razor-pages.md`, `ui-mvc.md` |
| APIs | `apis-minimal-and-controllers.md` |
| Real-time apps | `realtime-grpc-and-background-work.md` |
| Remote Procedure Call apps | `realtime-grpc-and-background-work.md` |
| Servers, Host and deploy | `testing-performance-and-operations.md` |
| Test, Debug, Troubleshoot | `testing-performance-and-operations.md` |
| Data access | `data-state-and-services.md` |
| Security and Identity | `security-and-identity.md` |
| Performance | `testing-performance-and-operations.md` |
| Migration and updates | `versioning-and-upgrades.md` |

## Areas To Consult Directly On Microsoft Learn

The following topics are part of the ASP.NET Core documentation tree but are not expanded into their own dedicated reference file here:

- globalization and localization
- advanced hosting and YARP details
- debugger and diagnostics tooling specifics
- narrow API-reference pages for individual types

When a task is dominated by one of those areas, go straight to the matching Microsoft Learn section after checking the reference files in this skill.

## Practical Deep-Dive Rule

- Start with the focused reference in this skill
- If the task depends on a narrow platform detail, open the matching Learn article
- If the task depends on version-specific behavior, confirm the correct moniker or breaking-changes page


---

<!-- merged from: razor-pages.md -->

﻿---
name: Razor Pages
description: # Razor Pages
 
 Primary docs:
---

# Razor Pages

Primary docs:

- <https://learn.microsoft.com/aspnet/core/razor-pages/>
- <https://learn.microsoft.com/aspnet/core/tutorials/razor-pages/>

## Choose Razor Pages For Page-Centered Apps

Prefer Razor Pages when requests naturally map to pages, forms, and page-level handlers. This is a strong default for internal tools, CRUD apps, account flows, and admin surfaces.

## Core Shape

Enable Razor Pages with:

- `builder.Services.AddRazorPages();`
- `app.MapRazorPages();`

Use the `@page` directive to turn a `.cshtml` file into an endpoint. Keep request logic in the paired `PageModel` class when the page is more than trivial.

## Routing Model

- File system location defines the route by default
- `Pages/Index.cshtml` maps to `/`
- `Pages/Store/Index.cshtml` maps to `/Store`
- Keep folder structure meaningful because it becomes the URL structure

## PageModel Guidance

- Use `OnGet`, `OnPost`, and named handlers for request processing
- Use bindable properties and model validation for forms
- Keep page models thin; move business logic into injected services
- Use Tag Helpers and model binding instead of manual request parsing

## Good Fits

- form-heavy workflows
- dashboards and back-office applications
- simple content with server-side validation
- applications where a page is the primary navigation unit

## Key Limitation

Do not rely on per-handler authorization with Razor Pages. Microsoft explicitly recommends using MVC controllers when different handlers on the same logical surface need different authorization behavior.

Preferred responses to that limitation:

- split the handlers into separate pages
- move the surface to MVC if action-level authorization is a better fit

## Organizational Guidance

- Group related pages into folders
- Use partial views for repeated fragments
- Use areas only when the application has clear bounded sections
- Keep shared layout and page conventions centralized


---

<!-- merged from: communitytoolkit-controls-and-helpers.md -->

﻿---
name: CommunityToolkit Controls and Helpers
description: ## What This Reference Is For
 
 Use this file when deciding whether the Windows Community Toolkit should be added to a WinUI 3 app.
tags: communitytoolkit, controls, helpers, animations, settingscontrols
---

## What This Reference Is For

Use this file when deciding whether the Windows Community Toolkit should be added to a WinUI 3 app.

## Prefer

- Platform controls first.
- Targeted Toolkit package additions for clear gaps such as richer settings surfaces, segmented controls, or focused animation helpers.
- The smallest package set that solves the problem.

## Avoid

- Adding Toolkit packages because they look convenient without checking whether WinUI already covers the need.
- Pulling in multiple Toolkit packages for a minor visual difference.
- Hiding fundamental UX problems behind a new dependency.

## Good Candidate Areas

- `SettingsControls`
  - useful for settings surfaces and cards
- `Segmented`
  - useful when segmented selection is clearer than a tab or radio cluster
- `HeaderedControls`
  - useful for labeled control groupings
- `Animations`
  - useful when built-in transitions are not enough
- helpers and extensions
  - useful when they reduce repetitive WinUI plumbing cleanly

## Package Guidance

- Prefer WinUI 3 compatible Toolkit packages.
- Add only what the app will actually use.
- Document why a Toolkit dependency was added and what built-in alternative was rejected.

## Sample and Source Anchors

- CommunityToolkit `components/SettingsControls`
- CommunityToolkit `components/Segmented`
- CommunityToolkit `components/HeaderedControls`
- Toolkit animations and helper packages

## Review Checklist

- Does built-in WinUI already solve the problem?
- Is the dependency narrowly scoped and justified?
- Does the new control match the rest of the app’s design language?
- Will the package meaningfully reduce custom code or improve UX?


---

<!-- merged from: windows-app-sdk-lifecycle-notifications-and-deployment.md -->

﻿---
name: Windows App SDK Lifecycle, Notifications, and Deployment
description: ## What This Reference Is For
 
 Use this file when the user needs lifecycle, activation, notification, packaged vs unpackaged, or runtime initialization guidance that goes beyond plain XAML UI work.
tags: windows-app-sdk, lifecycle, activation, notifications, deployment, packaged, unpackaged
---

## What This Reference Is For

Use this file when the user needs lifecycle, activation, notification, packaged vs unpackaged, or runtime initialization guidance that goes beyond plain XAML UI work.

## Prefer

- Learning the scenario from the matching WindowsAppSDK sample before designing an abstraction.
- Packaged deployment when it fits the product constraints.
- Explicit unpackaged guidance when the user has an installer, external-location requirement, or expects repeatable direct executable launches during development.

## Avoid

- Mixing packaged and unpackaged guidance in one answer without stating which path applies.
- Treating deployment requirements as optional details.
- Re-implementing lifecycle behavior already covered by Windows App SDK APIs.
- Using package-identity-dependent APIs in unpackaged startup code without an explicit guard or replacement path.

## Guidance

- Use AppLifecycle guidance and samples for activation, instancing, restart, and state notifications.
- Use notifications samples for push or app notifications rather than inventing custom delivery logic.
- For packaged apps, account for framework-dependent deployment and runtime package requirements.
- For unpackaged apps, account for bootstrapper and runtime initialization requirements.
- For unpackaged apps, treat package identity as absent unless the app deliberately establishes it through the chosen deployment model.
- Keep storage, settings, and startup services aligned with the deployment model. If a service assumes packaged storage or activation, redesign it before local unpackaged verification.
- Explain the deployment model before giving build or publish steps.

## Sample and Source Anchors

- WindowsAppSDK-Samples `Samples/AppLifecycle`
- WindowsAppSDK-Samples `Samples/Notifications`
- WindowsAppSDK-Samples `Samples/Unpackaged`
- WindowsAppSDK-Samples `Samples/CustomControls`
- Learn packaged and unpackaged deployment guides

## Review Checklist

- Is the app’s deployment model explicit?
- Are lifecycle and activation behaviors using platform APIs rather than ad hoc workarounds?
- Are notification requirements matched to the correct sample and runtime guidance?
- Does the recommendation match packaged or unpackaged constraints?

---

<!-- merged from: shell-navigation-and-windowing.md -->

﻿---
name: Shell, Navigation, and Windowing
description: ## What This Reference Is For
 
 Use this file for top-level app shells, page navigation models, custom title bars, and multi-window decisions.
tags: navigationview, titlebar, appwindow, multi-window, shell
---

## What This Reference Is For

Use this file for top-level app shells, page navigation models, custom title bars, and multi-window decisions.

## Prefer

- `NavigationView` for standard desktop shells with clear top-level destinations.
- A small, stable set of primary destinations.
- Built-in back navigation behavior that matches user expectations.
- `AppWindow` and Windows App SDK windowing APIs for modern window management.

## Avoid

- Overloading the nav surface with every command and secondary action.
- Turning the `NavigationView` pane into a branded hero area when the user did not ask for custom shell treatment.
- Custom title bar layouts that break drag regions or caption button clarity.
- Multi-window designs unless the workflow clearly benefits from them.

## Navigation Guidance

- Use left navigation when the app has several stable, high-level destinations.
- Use top navigation when there are few peer destinations and width is available.
- Use a single-page or document-first layout when navigation is shallow and the user mostly stays in one workflow.
- Keep naming and iconography stable across pages.
- Treat `NavigationView` as functional shell chrome first. Keep pane headers, footer content, and decorative branding minimal unless the product requirements clearly call for them.
- Prefer the platform's normal pane structure before adding custom logo blocks, taglines, or non-navigation content that changes the shell's native feel.
- For narrow or phone-like widths, stop reserving permanent pane width for desktop navigation. Prefer a minimal or overlay navigation mode, show the pane toggle when needed, close the pane by default after navigation, and give content the width back.
- When a shell enters a phone-width mode, reduce content padding and decorative chrome so the page reads as one primary column instead of a desktop shell with a squeezed content strip.

## Title Bar Guidance

- Treat the title bar as functional chrome first, branding surface second.
- Keep empty non-interactive areas draggable.
- Blend title bar visuals with the rest of the app when possible.
- Respect light, dark, and high-contrast states.

## Windowing Guidance

- Start with one main window.
- Add secondary windows only for workflows such as document detachment, inspection panes, or tool windows.
- Use Windows App SDK samples for resizing, placement, and window-specific behaviors instead of inventing custom platform abstractions.

## Sample and Source Anchors

- WinUI Gallery `NavigationView`, `TitleBar`, `AppWindow`, and windowing sample pages
- WindowsAppSDK-Samples `Samples/Windowing`
- Learn navigation and title bar guidance

## Review Checklist

- Is the navigation model simple and intentional?
- Does the shell still look and behave like a normal WinUI `NavigationView` unless there is an explicit reason to diverge?
- Does the title bar still behave like a Windows title bar?
- Are back, search, and pane behaviors consistent?
- Is multi-window use justified by the workflow?
- Does the shell intentionally switch behavior at narrow or phone widths instead of leaving a full desktop pane open?


---

<!-- merged from: setup-and-project-selection.md -->

﻿---
name: Setup and Project Selection
description: ## What This Reference Is For
 
 Use this file when the user is starting from scratch, choosing a project template, or asking what a WinUI machine needs before code work begins.
tags: setup, prerequisites, packaged, unpackaged, visual-studio, dotnet
---

## What This Reference Is For

Use this file when the user is starting from scratch, choosing a project template, or asking what a WinUI machine needs before code work begins.

## Prefer

- The setup-and-scaffold flow in [../SKILL.md](../SKILL.md) for prerequisite setup, template verification, and the first scaffold.
- A C# WinUI 3 desktop app on the Windows App SDK unless the user has a clear reason to prefer C++ or an existing non-WinUI stack.
- Official project templates and default packaging choices first.
- The current supported LTS .NET SDK for new C# work instead of only meeting the bare minimum.
- A packaged app by default for the smoothest first-project, deployment, and Store-compatible path.
- An unpackaged app when the user explicitly needs repeatable CLI build-and-run verification or direct executable launches as the normal local workflow.

## Avoid

- Starting project setup before the setup-and-scaffold flow in this skill has finished.
- Starting with unpackaged deployment unless the user needs repeatable CLI launch, an installer, existing desktop app integration, or a deliberate runtime strategy.
- Giving machine-readiness advice without verification.
- Treating old Windows builds, missing SDKs, or partial Visual Studio installs as "probably fine."
- Deferring the packaging choice until after startup, storage, and launch code are already written.

## Setup Baseline

- Use the setup-and-scaffold flow in [../SKILL.md](../SKILL.md) for prerequisite setup, template verification, and the first scaffold.
- Treat [../config.yaml](../config.yaml) as the bundled WinGet bootstrap source for setup and remediation.
- Return to this reference only after that workflow completes or when the task moves beyond initial project creation.
- Windows 10 version 1809 (build 17763) or later is the floor.
- Windows SDK 10.0.19041.0 or later is the practical baseline.
- Visual Studio with the WinUI application development workload is the supported primary IDE path.
- For C# apps, a supported .NET SDK must be installed.
- Developer Mode matters for common local deploy and debug flows.

## Project Selection Guidance

- Choose packaged when the user wants the default WinUI 3 path, easy local F5 workflows, or Store-friendly deployment. Keep the scaffold at its default unless the user explicitly asks for unpackaged behavior.
- Choose packaged when the app needs package identity or package-backed APIs during normal operation.
- Choose unpackaged when the user expects direct `.exe` launches, agent-driven local verification after each change, or integration with an existing installer or external location. Request that option through the setup flow instead of converting the initial project afterward.
- For either packaging model, scaffold first through the setup flow in `SKILL.md` and continue from the generated project instead of copying in prebuilt baseline files.
- If startup or shared resources later become suspect, create a fresh comparison app with the same packaging model and diff against that `dotnet new winui` output before broader restructuring.
- Once the model is chosen, keep startup and service code consistent with that model.
- Choose the standard blank app template first, then layer in navigation, title bar, or windowing patterns as the app matures.

## Sample and Source Anchors

- Learn `start-here.md` for the current official setup path.
- Learn `winui/winui3/index.md` for the framework position and platform benefits.
- Learn `windows-app-sdk/index.md` for the Windows App SDK feature surface.
- Learn `system-requirements.md` for tool and OS baselines.

## Review Checklist

- Is the machine baseline actually verified through the setup-and-scaffold flow in `SKILL.md`?
- Is the chosen packaging model intentional?
- Does the launch workflow match the chosen packaging model?
- Is the app still rooted in the standard WinUI template unless there is a real reason not to?
- Is the recommendation aligned with a C#-first WinUI 3 workflow?