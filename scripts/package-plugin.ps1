param(
    [string]$SourceDir = "plugin_samples/hello-hook",
    [string]$OutDir = "plugin_samples/dist"
)

$ErrorActionPreference = "Stop"

if (!(Test-Path $SourceDir)) {
    throw "Source plugin directory not found: $SourceDir"
}

if (!(Test-Path $OutDir)) {
    New-Item -ItemType Directory -Path $OutDir | Out-Null
}

$manifestPath = Join-Path $SourceDir "manifest.json"
if (!(Test-Path $manifestPath)) {
    throw "manifest.json not found in $SourceDir"
}

$manifest = Get-Content $manifestPath -Raw | ConvertFrom-Json
$pluginId = [string]$manifest.plugin_id
$version = [string]$manifest.version
if ([string]::IsNullOrWhiteSpace($pluginId) -or [string]::IsNullOrWhiteSpace($version)) {
    throw "manifest.json must contain plugin_id and version"
}

$zipName = "$pluginId-$version.zip"
$zipPath = Join-Path $OutDir $zipName
if (Test-Path $zipPath) {
    Remove-Item $zipPath -Force
}

Compress-Archive -Path (Join-Path $SourceDir "*") -DestinationPath $zipPath -CompressionLevel Optimal
Write-Output "Plugin package created: $zipPath"
