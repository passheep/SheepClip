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
  throw "未安装 GitHub CLI。请先执行：winget install GitHub.cli，然后执行：gh auth login"
}

if (-not (Test-Path $asset)) {
  throw "未找到安装包：$asset。请先执行：npm run tauri:build"
}

gh auth status
if ($LASTEXITCODE -ne 0) {
  throw "GitHub CLI 未登录或认证失败。请先执行：gh auth login"
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
  Write-Host "Release $Tag 已存在，上传并覆盖安装包..."
  gh release upload $Tag $asset --repo $Repo --clobber
  if ($LASTEXITCODE -ne 0) {
    throw "上传 Release 安装包失败：$Tag"
  }
} else {
  Write-Host "创建 Release $Tag 并上传安装包..."
  gh release create $Tag $asset @releaseArgs --title "SheepClip $Version" --generate-notes
  if ($LASTEXITCODE -ne 0) {
    throw "创建 Release 失败：$Tag"
  }
}

Write-Host "Release 发布完成：$Tag"
