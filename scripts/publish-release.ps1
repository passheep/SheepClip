param(
  [string]$Version = "",
  [string]$Repo = "passheep/SheepClip",
  [string]$Tag = "",
  [switch]$Draft,
  [switch]$Prerelease
)

$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $PSScriptRoot
Set-Location $root

if ([string]::IsNullOrWhiteSpace($Version)) {
  $package = Get-Content -Raw "package.json" | ConvertFrom-Json
  $Version = $package.version
}

if ([string]::IsNullOrWhiteSpace($Tag)) {
  $Tag = "v$Version"
}

$asset = Join-Path $root "src-tauri\target\release\bundle\nsis\SheepClip_${Version}_x64-setup.exe"

if (-not (Get-Command gh -ErrorAction SilentlyContinue)) {
  throw "GitHub CLI is not installed. Run: winget install GitHub.cli ; then run: gh auth login"
}

if (-not (Test-Path $asset)) {
  throw "Release asset not found: $asset. Run first: npm run tauri:build"
}

gh auth status
if ($LASTEXITCODE -ne 0) {
  throw "GitHub CLI is not authenticated. Run first: gh auth login"
}

$releaseArgs = @("--repo", $Repo)
if ($Draft) {
  $releaseArgs += "--draft"
}
if ($Prerelease) {
  $releaseArgs += "--prerelease"
}

gh release view $Tag --repo $Repo *> $null
if ($LASTEXITCODE -eq 0) {
  Write-Host "Release $Tag exists. Uploading asset with overwrite..."
  gh release upload $Tag $asset --repo $Repo --clobber
  if ($LASTEXITCODE -ne 0) {
    throw "Failed to upload release asset: $Tag"
  }
} else {
  Write-Host "Creating Release $Tag and uploading asset..."
  gh release create $Tag $asset @releaseArgs --title "SheepClip $Version" --generate-notes
  if ($LASTEXITCODE -ne 0) {
    throw "Failed to create release: $Tag"
  }
}

Write-Host "Release publish finished: $Tag"
