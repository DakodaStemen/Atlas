---
name: powershell-scripting
description: Authoritative patterns for writing production-quality PowerShell scripts and modules across Windows, Linux, and macOS using PS 7+.
domain: languages
category: scripting
tags: [PowerShell, PS7, scripting, automation, Windows, cross-platform, modules]
triggers: powershell script, PS7, pwsh, powershell function, powershell module, powershell error handling, powershell automation, write-ps, powershell best practices
---

# PowerShell Scripting

PowerShell is a task automation shell built on .NET. Since version 7, it runs cross-platform (Windows, Linux, macOS) via the `pwsh` binary. The language is object-oriented at its core: commands produce and consume typed .NET objects through the pipeline rather than text streams. This distinction drives most of the design patterns here—outputting raw text instead of objects, or building strings when you should pipe, are the most common causes of brittle scripts.

This document covers idiomatic PowerShell 7+ patterns. Where behavior differs between PS 7 and Windows PowerShell 5.1, that is called out explicitly.

---

## When to Use

- Automating system administration tasks on Windows, Linux, or macOS
- Orchestrating Azure, AWS, or on-premises infrastructure via their PowerShell modules
- Writing CI/CD pipeline scripts that must run on multiple OS runner types
- Building reusable automation modules for a team or product
- Replacing fragile batch files or shell scripts with robust, tested code

---

## Core Language Patterns

### Script Structure

Every non-trivial script follows this top-to-bottom layout:

```powershell
#Requires -Version 7.0
#Requires -Modules Az.Accounts

<#
.SYNOPSIS
    One-sentence description.

.DESCRIPTION
    Full explanation of what the script does, any side effects, and prerequisites.

.PARAMETER ComputerName
    The target host. Accepts pipeline input.

.EXAMPLE
    Get-TargetStatus -ComputerName 'srv01' -Verbose

.NOTES
    Author: Your Name
    Version: 1.0.0
#>

[CmdletBinding(SupportsShouldProcess = $true)]
param(
    [Parameter(Mandatory, ValueFromPipeline, ValueFromPipelineByPropertyName)]
    [ValidateNotNullOrEmpty()]
    [string[]]$ComputerName,

    [Parameter()]
    [ValidateRange(1, 300)]
    [uint32]$TimeoutSeconds = 30
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# Function definitions go here, before main execution logic.

# Main execution at the bottom.
```

Key rules:

- `#Requires` statements must be the very first lines before comments or blank lines.
- `[CmdletBinding()]` on every advanced function and at the script level.
- `Set-StrictMode -Version Latest` catches use of uninitialized variables.
- `$ErrorActionPreference = 'Stop'` converts non-terminating errors to terminating ones so `try/catch` can handle them.

### Naming Conventions

| Element | Convention | Example |
| --- | --- | --- |
| Functions / cmdlets | PascalCase Verb-Noun | `Get-DeploymentStatus` |
| Parameters | PascalCase | `$ComputerName` |
| Private/local variables | camelCase | `$resolvedPath` |
| Module names | PascalCase | `MyCompany.Ops` |
| Constants | UPPER_SNAKE or PascalCase | `$MaxRetryCount` |
| Keywords | lowercase | `foreach`, `if`, `try` |
| Operators | lowercase | `-eq`, `-match`, `-and` |

Always use approved verbs. Check with `Get-Verb`. Using an unapproved verb causes an import warning and breaks discoverability. Common pairs: `Get`/`Set`, `New`/`Remove`, `Add`/`Clear`, `Enable`/`Disable`, `Start`/`Stop`, `Invoke`/`Suspend`.

### Functions and Advanced Functions

A basic function becomes an advanced function by adding `[CmdletBinding()]`. This unlocks `-Verbose`, `-Debug`, `-ErrorAction`, `-WhatIf`, `-Confirm`, and the common parameters automatically.

```powershell
function Invoke-Deployment {
    [CmdletBinding(SupportsShouldProcess = $true, ConfirmImpact = 'High')]
    param(
        [Parameter(Mandatory)]
        [ValidateNotNullOrEmpty()]
        [string]$Environment,

        [Parameter()]
        [ValidateSet('Blue', 'Green', 'Canary')]
        [string]$Strategy = 'Blue',

        [Parameter()]
        [switch]$Force
    )

    begin {
        Write-Verbose "Starting deployment to $Environment (strategy: $Strategy)"
    }

    process {
        if ($PSCmdlet.ShouldProcess($Environment, 'Deploy application')) {
            # actual work here
        }
    }

    end {
        Write-Verbose "Deployment block complete"
    }
}
```

Use `begin`/`process`/`end` blocks whenever the function accepts pipeline input. `begin` runs once before pipeline objects arrive; `process` runs once per piped object; `end` runs once after all input is consumed.

### Parameter Validation

Prefer declarative validation attributes over manual `if` guards:

```powershell
param(
    [ValidateNotNullOrEmpty()]
    [string]$Name,

    [ValidateRange(1, 65535)]
    [int]$Port,

    [ValidateSet('dev', 'staging', 'prod')]
    [string]$Stage,

    [ValidatePattern('^[a-zA-Z0-9\-]+$')]
    [string]$Slug,

    [ValidateScript({ Test-Path $_ -PathType Leaf })]
    [string]$ConfigFile,

    # For booleans, use [switch] not [bool]
    [switch]$Force
)
```

`[ValidateScript]` lets you express arbitrary conditions. Throw a descriptive message from inside it:

```powershell
[ValidateScript({
    if (-not (Test-Path $_)) { throw "Path '$_' does not exist." }
    $true
})]
[string]$InputPath
```

### Pipeline

Write functions that work in the middle of a pipeline, not just as entry points. Accept `ValueFromPipeline` on the key parameter and emit objects one at a time—do not buffer into an array and return it at the end.

```powershell
function Get-ServiceHealth {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory, ValueFromPipeline, ValueFromPipelineByPropertyName)]
        [string]$Name
    )

    process {
        $svc = Get-Service -Name $Name -ErrorAction SilentlyContinue
        if ($svc) {
            [PSCustomObject]@{
                Name   = $svc.Name
                Status = $svc.Status
                StartType = $svc.StartType
            }
        } else {
            Write-Warning "Service '$Name' not found."
        }
    }
}

# Usage:
'wuauserv', 'bits', 'spooler' | Get-ServiceHealth | Where-Object Status -eq 'Running'
```

Always output `[PSCustomObject]` with explicit property names rather than `Write-Host` or concatenated strings. Downstream commands can then filter, sort, and export those objects.

---

## Modules and Reusability

### Module Layout

```text
MyModule/
├── MyModule.psd1          # Module manifest (required)
├── MyModule.psm1          # Root module loader
├── Public/
│   ├── Get-Thing.ps1
│   └── Set-Thing.ps1
└── Private/
    └── Invoke-InternalHelper.ps1
```

The `.psm1` dot-sources all files and exports only public functions:

```powershell
# MyModule.psm1
$publicFunctions  = Get-ChildItem -Path "$PSScriptRoot/Public"  -Filter '*.ps1'
$privateFunctions = Get-ChildItem -Path "$PSScriptRoot/Private" -Filter '*.ps1'

foreach ($file in $privateFunctions + $publicFunctions) {
    . $file.FullName
}

Export-ModuleMember -Function ($publicFunctions.BaseName)
```

### Module Manifest

Generate with `New-ModuleManifest`, then edit the key fields:

```powershell
New-ModuleManifest -Path './MyModule/MyModule.psd1' `
    -RootModule 'MyModule.psm1' `
    -ModuleVersion '1.0.0' `
    -Author 'Your Name' `
    -Description 'What this module does' `
    -PowerShellVersion '7.0' `
    -FunctionsToExport @('Get-Thing', 'Set-Thing') `
    -RequiredModules @()
```

Always set `FunctionsToExport` explicitly. The wildcard `'*'` exports everything including private helpers.

### Dependency Management

```powershell
#Requires -Modules @{ ModuleName = 'Az.Accounts'; ModuleVersion = '2.0.0' }
```

For team repositories, commit a `requirements.psd1` and use `Install-PSResource` (PS 7.4+) or `Install-Module -Scope CurrentUser` in bootstrap scripts.

---

## Error Handling and Robustness

### Terminating vs. Non-Terminating Errors

Most cmdlets emit non-terminating errors—they write to the error stream but continue executing. `try/catch` only intercepts terminating errors. To catch errors from cmdlets, convert them:

```powershell
# Option 1: per-call
Get-Item 'C:\nonexistent' -ErrorAction Stop

# Option 2: script-wide (set at the top)
$ErrorActionPreference = 'Stop'
```

### try / catch / finally

```powershell
try {
    $connection = Connect-Database -Server $Server -ErrorAction Stop
    $result = Invoke-Query -Connection $connection -Query $sql
    $result
} catch [System.Data.SqlClient.SqlException] {
    Write-Error "SQL error on $Server : $_"
    throw  # re-throw if the caller should know
} catch {
    Write-Error "Unexpected error: $($_.Exception.Message)"
    Write-Verbose "Stack trace: $($_.ScriptStackTrace)"
    throw
} finally {
    if ($connection) { $connection.Dispose() }
}
```

Rules:

- Catch specific exception types first, then the general `catch` last.
- Never swallow exceptions with an empty `catch {}` unless you log and have a deliberate reason.
- Use `finally` for cleanup (disposing connections, removing temp files) regardless of success or failure.
- Re-throw with bare `throw` (not `throw $_`) to preserve the original stack trace.

### Retry Logic

```powershell
function Invoke-WithRetry {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory)]
        [scriptblock]$ScriptBlock,
        [int]$MaxAttempts = 3,
        [int]$DelaySeconds = 5
    )

    $attempt = 0
    do {
        $attempt++
        try {
            & $ScriptBlock
            return
        } catch {
            if ($attempt -ge $MaxAttempts) { throw }
            Write-Warning "Attempt $attempt failed: $_. Retrying in ${DelaySeconds}s..."
            Start-Sleep -Seconds $DelaySeconds
        }
    } while ($true)
}
```

### Structured Logging

Do not use `Write-Host` for information you want callers to capture. Use the correct stream:

| Stream | Cmdlet | Use for |
| --- | --- | --- |
| Output (1) | `Write-Output` / implicit return | Data objects for pipeline |
| Error (2) | `Write-Error` | Non-fatal errors |
| Warning (3) | `Write-Warning` | Surprising but recoverable conditions |
| Verbose (4) | `Write-Verbose` | Execution details under `-Verbose` |
| Debug (5) | `Write-Debug` | Low-level diagnostic info |
| Information (6) | `Write-Information` | User-facing informational messages |

`Write-Host` bypasses all streams and cannot be redirected or captured. Reserve it for interactive prompts or colored output in console-only scripts.

---

## Critical Rules and Gotchas

**Never use aliases in scripts.** `%`, `?`, `gci`, `ls`, `dir` are fine interactively, but break cross-platform compatibility and readability. Always use full cmdlet names.

**Avoid `Invoke-Expression`.** It is an injection vector. If you find yourself building a command string to execute, refactor to use parameter splatting or script blocks instead.

**Do not use `+=` to build arrays in loops.** Every `+=` on an array creates a new array in memory, making the loop O(n²). Use a `List<T>` or capture pipeline output directly:

```powershell
# Wrong — O(n²)
$results = @()
foreach ($item in $collection) { $results += Process-Item $item }

# Right — O(n)
$results = [System.Collections.Generic.List[PSObject]]::new()
foreach ($item in $collection) { $results.Add((Process-Item $item)) }

# Also right — collect pipeline output
$results = foreach ($item in $collection) { Process-Item $item }
```

**Use `$PSCmdlet.ShouldProcess()` for destructive operations.** This enables `-WhatIf` and `-Confirm` automatically when `SupportsShouldProcess = $true` is set in `[CmdletBinding()]`.

**Use `[PSCredential]` for credentials, never plaintext strings.** Use `Get-Credential` interactively or the `SecretManagement` module for vault-backed secrets at runtime.

**Cross-platform path separators.** Use `Join-Path` instead of string concatenation with `\` or `/`. On PS 7, `$PSScriptRoot`, `[IO.Path]::Combine()`, and `Join-Path` all normalize correctly.

### Check platform when needed, but prefer capability checks

```powershell
# Capability check (preferred)
if (Get-Command 'systemctl' -ErrorAction SilentlyContinue) {
    systemctl status nginx
}

# Platform check (use sparingly)
if ($IsWindows) {
    # Windows-only code
} elseif ($IsLinux) {
    # Linux-only code
}
```

`$IsWindows`, `$IsLinux`, `$IsMacOS` are PS 6+ only. In PS 5.1, they do not exist—guard with `$PSVersionTable.PSVersion`.

**Replace WMI with CIM.** `Get-WmiObject` is deprecated and Windows-only. Use `Get-CimInstance` everywhere; it works over WSMAN and is cross-platform capable.

**`Set-StrictMode -Version Latest` will break legacy scripts** that rely on accessing undefined variables or uninitialized properties. Enable it in new code, not when porting old code without testing.

**Splatting for long parameter lists.** Backtick line continuation is fragile (invisible trailing space breaks it). Use splatting instead:

```powershell
$params = @{
    ComputerName = $server
    Credential   = $cred
    Port         = 5985
    UseSSL       = $true
}
New-PSSession @params
```

---

## Key Commands Reference

| Purpose | Command |
| --- | --- |
| List approved verbs | `Get-Verb` |
| Check script for issues | `Invoke-ScriptAnalyzer -Path ./script.ps1` |
| PS version info | `$PSVersionTable` |
| Find commands | `Get-Command -Noun Process` |
| Module info | `Get-Module -ListAvailable ModuleName` |
| Install module (user) | `Install-PSResource -Name Az -Scope CurrentUser` |
| Generate manifest | `New-ModuleManifest -Path ./Mod/Mod.psd1` |
| Run Pester tests | `Invoke-Pester -Path ./tests/` |
| Profile secrets | `Get-SecretInfo` (requires `SecretManagement`) |
| CIM query | `Get-CimInstance -ClassName Win32_OperatingSystem` |
| Format as table | `$obj \ | Format-Table -AutoSize` |
| Export to CSV | `$obj \ | Export-Csv -Path out.csv -NoTypeInformation` |
| Convert to JSON | `$obj \ | ConvertTo-Json -Depth 5` |

---

## References

- [PowerShell Documentation (Microsoft Learn)](https://learn.microsoft.com/en-us/powershell/)
- [Strongly Encouraged Development Guidelines](https://learn.microsoft.com/en-us/powershell/scripting/developer/cmdlet/strongly-encouraged-development-guidelines?view=powershell-7.5)
- [PowerShell Practice and Style Guide (PoshCode)](https://poshcode.gitbook.io/powershell-practice-and-style/)
- [About Functions Advanced (Microsoft Learn)](https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.core/about/about_functions_advanced?view=powershell-7.5)
- [PSScriptAnalyzer (GitHub)](https://github.com/PowerShell/PSScriptAnalyzer)
- [Pester Testing Framework](https://pester.dev)
- [SecretManagement Module](https://learn.microsoft.com/en-us/powershell/utility-modules/secretmanagement/overview?view=ps-modules)
- [PowerShell 7 Cross-Platform Tips (Jeff Hicks)](https://jdhitsolutions.com/blog/scripting/7361/powershell-7-cross-platform-scripting-tips-and-traps/)
- [Enterprise PowerShell Best Practices (dstreefkerk)](https://dstreefkerk.github.io/2025-06-powershell-scripting-best-practices/)
