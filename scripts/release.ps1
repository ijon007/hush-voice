<#
.SYNOPSIS
  Build and publish a Hush release on GitHub.

.EXAMPLE
  .\scripts\release.ps1                 # release the version in Cargo.toml
  .\scripts\release.ps1 -Version 0.2.0  # bump Cargo.toml, commit, then release
  .\scripts\release.ps1 -Draft          # create the GitHub release as a draft

  Steps: (bump) -> cargo build --release (MSVC env) -> Inno Setup installer
  -> commit + tag vX.Y.Z -> push -> gh release create with the installer.
  Needs: Visual Studio 2022 C++ tools, LLVM, CMake, Inno Setup 6, gh (logged in).
#>
param(
    [string]$Version,
    [switch]$Draft
)
$ErrorActionPreference = 'Stop'
$root = Split-Path $PSScriptRoot -Parent
Set-Location $root

function Fail($msg) { Write-Host "release: $msg" -ForegroundColor Red; exit 1 }

# --- tools -------------------------------------------------------------------
$vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
$vs = & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
$vcvars = Join-Path $vs 'VC\Auxiliary\Build\vcvars64.bat'
if (-not (Test-Path $vcvars)) { Fail "vcvars64.bat not found (install VS 2022 C++ tools)" }
$iscc = @(
    "$env:LOCALAPPDATA\Programs\Inno Setup 6\ISCC.exe",
    "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe",
    "$env:ProgramFiles\Inno Setup 6\ISCC.exe"
) | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $iscc) { Fail "Inno Setup 6 not found (winget install JRSoftware.InnoSetup)" }
if (-not (Get-Command gh -ErrorAction SilentlyContinue)) { Fail "gh not found (winget install GitHub.cli)" }
gh auth status *> $null
if ($LASTEXITCODE -ne 0) { Fail "gh is not logged in (run: gh auth login)" }

# --- version -----------------------------------------------------------------
$toml = Get-Content Cargo.toml -Raw
$current = [regex]::Match($toml, '(?m)^version = "([^"]+)"').Groups[1].Value
if (-not $current) { Fail "no version in Cargo.toml" }
if (git status --porcelain) { Fail "working tree is not clean; commit or stash first" }
$bump = $Version -and $Version -ne $current
if (-not $Version) { $Version = $current }
if ($Version -notmatch '^\d+\.\d+\.\d+$') { Fail "version must look like 1.2.3 (got '$Version')" }
$tag = "v$Version"
if (git tag --list $tag) { Fail "tag $tag already exists" }
if ($bump) {
    $re = [regex]'(?m)^version = "[^"]+"'
    $toml = $re.Replace($toml, "version = `"$Version`"", 1)
    [IO.File]::WriteAllText((Join-Path $root 'Cargo.toml'), $toml)
    Write-Host "Version $current -> $Version"
} else {
    Write-Host "Version $Version"
}

# --- build -------------------------------------------------------------------
$env:LIBCLANG_PATH = 'C:\Program Files\LLVM\bin'
$env:PATH = "C:\Program Files\CMake\bin;C:\Program Files\LLVM\bin;$env:PATH"
cmd /c "`"$vcvars`" >nul && cargo build --release"
if ($LASTEXITCODE -ne 0) { Fail "cargo build --release failed" }

& $iscc "/DAppVersion=$Version" installer\hush.iss
if ($LASTEXITCODE -ne 0) { Fail "installer build failed" }
$setup = Join-Path $root "installer\Output\Hush-Setup-$Version.exe"
if (-not (Test-Path $setup)) { Fail "installer not found at $setup" }

# --- tag + publish -------------------------------------------------------------
if ($bump) {
    git add Cargo.toml Cargo.lock
    git commit -m "Release $tag"
    if ($LASTEXITCODE -ne 0) { Fail "commit failed" }
}
git tag -a $tag -m "Hush $Version"
git push origin HEAD
git push origin $tag
if ($LASTEXITCODE -ne 0) { Fail "push failed" }

$ghArgs = @('release', 'create', $tag, $setup, '--title', "Hush $Version", '--generate-notes')
if ($Draft) { $ghArgs += '--draft' }
gh @ghArgs
if ($LASTEXITCODE -ne 0) { Fail "gh release create failed" }
Write-Host "Released $tag" -ForegroundColor Green
