[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [ValidatePattern('^\d+\.\d+\.\d+$')]
    [string]$Version,
    [string]$Remote = 'origin',
    [string]$Branch = 'main'
)

$ErrorActionPreference = 'Stop'

$repositoryRoot = Split-Path -Parent $PSScriptRoot
Set-Location $repositoryRoot

$tag = "release-v$Version"
$releaseSubject = "release(v$Version): publish desktop artifacts"
$pubspecPath = Join-Path $repositoryRoot 'pubspec.yaml'

function Invoke-Git([string[]]$Arguments) {
    & git @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "git $($Arguments -join ' ') failed with exit code $LASTEXITCODE."
    }
}

$currentBranch = (& git branch --show-current).Trim()
if ($LASTEXITCODE -ne 0) {
    throw 'Could not determine the current Git branch.'
}
if ($currentBranch -ne $Branch) {
    throw "Release must run from '$Branch'; current branch is '$currentBranch'."
}

$status = @(git status --porcelain)
if ($LASTEXITCODE -ne 0) {
    throw 'Could not inspect the Git working tree.'
}
if ($status.Count -gt 0) {
    throw 'Working tree is not clean. Commit or remove changes before publishing.'
}

& git rev-parse --verify --quiet "refs/tags/$tag" *> $null
if ($LASTEXITCODE -eq 0) {
    throw "Local tag '$tag' already exists. Choose a new version."
}

& git ls-remote --exit-code --tags $Remote "refs/tags/$tag" *> $null
$remoteTagExitCode = $LASTEXITCODE
if ($remoteTagExitCode -eq 0) {
    throw "Remote tag '$tag' already exists. Choose a new version."
}
if ($remoteTagExitCode -ne 2) {
    throw "Could not check remote tag '$tag' on '$Remote'."
}

if (-not (Test-Path -LiteralPath $pubspecPath -PathType Leaf)) {
    throw "Missing $pubspecPath."
}
$pubspec = [System.IO.File]::ReadAllText($pubspecPath)
$versionPattern = '(?m)^version:\s+(\d+\.\d+\.\d+)(?:\+(\d+))?\s*$'
$versionMatch = [regex]::Match($pubspec, $versionPattern)
if (-not $versionMatch.Success) {
    throw 'Could not find a valid version entry in pubspec.yaml.'
}
$targetVersion = "$Version+1"
if ($versionMatch.Value -match [regex]::Escape($targetVersion)) {
    throw "pubspec.yaml is already set to $targetVersion."
}

$updatedPubspec = [regex]::Replace(
    $pubspec,
    $versionPattern,
    "version: $targetVersion",
    1)
$utf8NoBom = New-Object -TypeName System.Text.UTF8Encoding -ArgumentList $false
[System.IO.File]::WriteAllText($pubspecPath, $updatedPubspec, $utf8NoBom)

try {
    Invoke-Git @('diff', '--check')
    & dart run tool/verify.dart
    if ($LASTEXITCODE -ne 0) {
        throw "dart run tool/verify.dart failed with exit code $LASTEXITCODE."
    }

    Invoke-Git @('add', 'pubspec.yaml')
    Invoke-Git @('commit', '-m', $releaseSubject)
    Invoke-Git @('tag', '-a', $tag, '-m', $releaseSubject)

    # Push exactly the release branch and tag. Never use --tags here.
    Invoke-Git @('push', $Remote, $Branch, $tag)
    Write-Output "Published $tag from $releaseSubject."
} catch {
    Write-Error $_
    throw
}
