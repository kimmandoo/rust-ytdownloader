param(
    [string]$ReleaseDirectory,
    [string]$OutputDirectory = "release-assets"
)

$ErrorActionPreference = "Stop"

if ([string]::IsNullOrWhiteSpace($ReleaseDirectory)) {
    $ReleaseDirectory = $env:ARTIFACT_PATH
}
if ([string]::IsNullOrWhiteSpace($ReleaseDirectory)) {
    throw "ReleaseDirectory or ARTIFACT_PATH is required."
}

$releasePath = (Resolve-Path $ReleaseDirectory).Path
if (-not (Test-Path (Join-Path $releasePath "spull.exe"))) {
    throw "The Windows release directory does not contain spull.exe: $releasePath"
}

$packageRoot = [System.IO.Path]::GetFullPath($OutputDirectory)
New-Item -ItemType Directory -Path $packageRoot -Force | Out-Null

$portableArchive = Join-Path $packageRoot "spull-windows-x86_64.zip"
$setupArchive = Join-Path $packageRoot "spull-windows-x86_64-setup.zip"
$temporaryDirectory = Join-Path ([System.IO.Path]::GetTempPath()) "spull-package-$PID"
$runtimeArchive = Join-Path $temporaryDirectory "spull-runtime.zip"
$setupStage = Join-Path $temporaryDirectory "setup"

try {
    New-Item -ItemType Directory -Path $temporaryDirectory -Force | Out-Null
    New-Item -ItemType Directory -Path $setupStage -Force | Out-Null

    Compress-Archive -Path (Join-Path $releasePath "*") `
        -DestinationPath $portableArchive -CompressionLevel Optimal -Force
    Compress-Archive -Path (Join-Path $releasePath "*") `
        -DestinationPath $runtimeArchive -CompressionLevel Optimal -Force

    Copy-Item -LiteralPath (Join-Path $PSScriptRoot "Install-Spull.ps1") `
        -Destination $setupStage -Force
    Copy-Item -LiteralPath (Join-Path $PSScriptRoot "Install-Spull.vbs") `
        -Destination $setupStage -Force
    Copy-Item -LiteralPath $runtimeArchive -Destination $setupStage -Force
    @'
Spull Setup
===========

Double-click Install-Spull.vbs to open the GUI installation wizard.
The launcher starts Windows PowerShell without opening a console window.
'@ | Set-Content -Path (Join-Path $setupStage "README.txt") -Encoding UTF8

    Compress-Archive -Path (Join-Path $setupStage "*") `
        -DestinationPath $setupArchive -CompressionLevel Optimal -Force

    Write-Output "Created $portableArchive"
    Write-Output "Created $setupArchive"
}
finally {
    if (Test-Path $temporaryDirectory) {
        Remove-Item -Recurse -Force $temporaryDirectory
    }
}
