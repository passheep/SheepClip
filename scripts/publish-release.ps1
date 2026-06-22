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

function Quote-ProcessArgument {
  param([string]$Value)

  if ($Value -match '^[A-Za-z0-9_\-\.\/\\:=]+$') {
    return $Value
  }

  return '"' + ($Value -replace '"', '\"') + '"'
}

function Invoke-GhCommand {
  param([string[]]$Arguments)

  $startInfo = New-Object System.Diagnostics.ProcessStartInfo
  $startInfo.FileName = "gh"
  $startInfo.Arguments = ($Arguments | ForEach-Object { Quote-ProcessArgument $_ }) -join " "
  $startInfo.UseShellExecute = $false
  $startInfo.RedirectStandardOutput = $true
  $startInfo.RedirectStandardError = $true

  $process = New-Object System.Diagnostics.Process
  $process.StartInfo = $startInfo
  [void]$process.Start()
  $stdout = $process.StandardOutput.ReadToEnd()
  $stderr = $process.StandardError.ReadToEnd()
  $process.WaitForExit()

  $text = (($stdout, $stderr) -join "`n").Trim()
  [pscustomobject]@{
    ExitCode = $process.ExitCode
    Stdout = $stdout
    Stderr = $stderr
    Text = $text
  }
}

function Get-ReleaseNotes {
  param(
    [string]$DisplayVersion,
    [string]$AssetName
  )

  $templateBase64 = "IyBTaGVlcENsaXAgX19WRVJTSU9OX18g5LiK57q/CgpTaGVlcENsaXAg5paw54mI5pysIF9fVkVSU0lPTl9fIOato+W8j+WPkeW4g+OAguacrOasoeabtOaWsOe7p+e7reWbtOe7leOAjOabtOW/q+WkjeWItuOAgeabtOW/q+i+k+WFpeOAgeabtOeos+WumuS9v+eUqOOAjei/m+ihjOS8mOWMlu+8jOW7uuiuruaJgOacieeUqOaIt+S4i+i9veacgOaWsCBXaW5kb3dzIOWuieijheWMheWNh+e6p+S9k+mqjOOAggoKIyMg5pu05paw5Lqu54K5CgotIOS8mOWMluWJqui0tOadv+WOhuWPsuS4juW/q+aNt+i+k+WFpeebuOWFs+S9k+mqjO+8jOWHj+WwkemHjeWkjeaTjeS9nO+8jOaPkOWNh+aXpeW4uOaWh+Wtl+i+k+WFpeaViOeOh+OAggotIOaUuei/m+eVjOmdouS4u+mimOOAgeWtl+S9k+WSjOa7muWKqOadoeetiee7huiKgu+8jOiuqeS4jeWQjOS4u+mimOS4i+eahOaYvuekuuabtOWKoOe7n+S4gOiHqueEtuOAggotIOS/ruWkjeW3suefpemXrumimO+8jOaPkOWNh+WQr+WKqOOAgeiuvue9ruS/neWtmOOAgeWuieijheWNh+e6p+etieWcuuaZr+eahOeos+WumuaAp+OAggotIOabtOaWsCBXaW5kb3dzIOWuieijheWMhe+8jOWPr+ebtOaOpeimhuebluaXp+eJiOacrOWuieijheS9v+eUqOOAggoKIyMg5LiL6L295pa55byPCgror7flnKjkuIvmlrkgQXNzZXRzIOS4reS4i+i9vSBXaW5kb3dzIOWuieijheWMhe+8mgoKLSBfX0FTU0VUX05BTUVfXwoKIyMg5a6J6KOF5bu66K6uCgotIOWNh+e6p+WJjeW7uuiuruWFiOmAgOWHuuato+WcqOi/kOihjOeahCBTaGVlcENsaXDjgIIKLSDlt7Llronoo4Xml6fniYjmnKznmoTnlKjmiLfvvIzlj6/ku6Xnm7TmjqXov5DooYzmlrDlronoo4XljIXopobnm5bljYfnuqfjgIIKLSDlpoLmnpwgV2luZG93cyDlronlhajkuK3lv4Plh7rnjrDmj5DnpLrvvIzor7fnoa7orqTlronoo4XljIXkuIvovb3mnaXmupDkuLrmnKzpobnnm64gR2l0SHViIFJlbGVhc2Vz44CCCgojIyDlj43ppojkuI7lu7rorq4KCuWmguaenOmBh+WIsOmXrumimO+8jOaIluacieaWsOeahOaWh+Wtl+i+k+WFpeaViOeOh+mcgOaxgu+8jOasoui/juWcqCBHaXRIdWIgSXNzdWVzIOS4reWPjemmiOOAggo="
  $template = [System.Text.Encoding]::UTF8.GetString([System.Convert]::FromBase64String($templateBase64))
  $template = $template.Replace("__VERSION__", $DisplayVersion)
  $template.Replace("__ASSET_NAME__", $AssetName)
}

if ([string]::IsNullOrWhiteSpace($Version)) {
  $package = Get-Content -Raw "package.json" | ConvertFrom-Json
  $Version = $package.version
}

if ([string]::IsNullOrWhiteSpace($Tag)) {
  $Tag = "v$Version"
}

$asset = Join-Path $root "src-tauri\target\release\bundle\nsis\SheepClip_${Version}_x64-setup.exe"
$assetName = Split-Path -Leaf $asset
$displayVersion = $Version -replace '\.0$', ''

if (-not (Get-Command gh -ErrorAction SilentlyContinue)) {
  throw "GitHub CLI is not installed. Run: winget install GitHub.cli ; then run: gh auth login"
}

if (-not (Test-Path $asset)) {
  throw "Release asset not found: $asset. Run first: npm run tauri:build"
}

$authStatus = Invoke-GhCommand @("auth", "status")
if ($authStatus.Text) {
  Write-Host $authStatus.Text
}
if ($authStatus.ExitCode -ne 0) {
  throw "GitHub CLI is not authenticated. Run first: gh auth login"
}

$releaseArgs = @("--repo", $Repo)
if ($Draft) {
  $releaseArgs += "--draft"
}
if ($Prerelease) {
  $releaseArgs += "--prerelease"
}
$notesPath = Join-Path $env:TEMP "sheepclip-release-notes-$Version.md"
[System.IO.File]::WriteAllText($notesPath, (Get-ReleaseNotes $displayVersion $assetName), [System.Text.Encoding]::UTF8)

$releaseView = Invoke-GhCommand @("release", "view", $Tag, "--repo", $Repo)
if ($releaseView.ExitCode -eq 0) {
  Write-Host "Release $Tag exists. Updating notes and uploading asset with overwrite..."
  $edit = Invoke-GhCommand @("release", "edit", $Tag, "--repo", $Repo, "--title", "SheepClip $displayVersion", "--notes-file", $notesPath)
  if ($edit.Text) {
    Write-Host $edit.Text
  }
  if ($edit.ExitCode -ne 0) {
    throw "Failed to update release notes: $Tag. $($edit.Text)"
  }
  $upload = Invoke-GhCommand @("release", "upload", $Tag, $asset, "--repo", $Repo, "--clobber")
  if ($upload.Text) {
    Write-Host $upload.Text
  }
  if ($upload.ExitCode -ne 0) {
    throw "Failed to upload release asset: $Tag. $($upload.Text)"
  }
} else {
  if ($releaseView.Text -and $releaseView.Text -notmatch "release not found") {
    throw "Failed to check release $Tag. $($releaseView.Text)"
  }
  Write-Host "Creating Release $Tag and uploading asset..."
  $create = Invoke-GhCommand (@("release", "create", $Tag, $asset) + $releaseArgs + @("--title", "SheepClip $displayVersion", "--notes-file", $notesPath))
  if ($create.Text) {
    Write-Host $create.Text
  }
  if ($create.ExitCode -ne 0) {
    throw "Failed to create release: $Tag. $($create.Text)"
  }
}

Write-Host "Release publish finished: $Tag"
