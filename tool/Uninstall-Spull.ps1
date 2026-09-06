param(
    [string]$RemovePath,
    [switch]$SkipConfirmation
)

$ErrorActionPreference = 'Stop'

Add-Type -AssemblyName System.Windows.Forms

function Show-Message(
    [string]$message,
    [string]$title,
    [System.Windows.Forms.MessageBoxButtons]$buttons,
    [System.Windows.Forms.MessageBoxIcon]$icon
) {
    return [System.Windows.Forms.MessageBox]::Show(
        $message,
        $title,
        $buttons,
        $icon)
}

function Quote-Argument([string]$value) {
    return '"' + $value.Replace('"', '\"') + '"'
}
function Remove-ShortcutIfTarget(
    [string]$path,
    [string]$target,
    [string]$arguments = ''
) {
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        return
    }
    try {
        $shell = New-Object -ComObject WScript.Shell
        $shortcut = $shell.CreateShortcut($path)
        $targetMatches = $shortcut.TargetPath -ieq $target
        $argumentsMatch = [string]::IsNullOrWhiteSpace($arguments) -or
            $shortcut.Arguments.Trim('" ') -ieq $arguments.Trim('" ')
        if ($targetMatches -and $argumentsMatch) {
            Remove-Item -LiteralPath $path -Force
        }
    } catch {
        # A stale or inaccessible shortcut must not block uninstall.
    }
}

$scriptPath = $MyInvocation.MyCommand.Definition
$installPath = if ([string]::IsNullOrWhiteSpace($RemovePath)) {
    Split-Path -Parent $scriptPath
} else {
    [System.IO.Path]::GetFullPath($RemovePath)
}
$programs = Join-Path ([Environment]::GetFolderPath('ApplicationData')) 'Microsoft\Windows\Start Menu\Programs'
$shortcutPath = Join-Path $programs 'Spull.lnk'
$wscript = Join-Path $env:WINDIR 'System32\wscript.exe'
$uninstallShortcutPath = Join-Path $programs 'Uninstall Spull.lnk'
$executable = Join-Path $installPath 'spull.exe'

if ([string]::IsNullOrWhiteSpace($RemovePath)) {
    if (-not (Test-Path -LiteralPath $installPath -PathType Container)) {
        Show-Message `
            'Spull is not installed at the expected location.' `
            'Spull Uninstall' `
            ([System.Windows.Forms.MessageBoxButtons]::OK) `
            ([System.Windows.Forms.MessageBoxIcon]::Information) | Out-Null
        exit 0
    }
    if (-not (Test-Path -LiteralPath $executable -PathType Leaf)) {
        Show-Message `
            'Spull is not installed at the expected location.' `
            'Spull Uninstall' `
            ([System.Windows.Forms.MessageBoxButtons]::OK) `
            ([System.Windows.Forms.MessageBoxIcon]::Information) | Out-Null
        exit 0
    }

    if (-not $SkipConfirmation) {
        $answer = Show-Message `
            'Remove Spull and its installed files?' `
            'Spull Uninstall' `
            ([System.Windows.Forms.MessageBoxButtons]::YesNo) `
            ([System.Windows.Forms.MessageBoxIcon]::Question)
        if ($answer -ne [System.Windows.Forms.DialogResult]::Yes) {
            exit 0
        }
    }

    $temporaryScript = Join-Path $env:TEMP "spull-uninstall-$([guid]::NewGuid().ToString('N')).ps1"
    Copy-Item -LiteralPath $scriptPath -Destination $temporaryScript -Force
    $arguments = @(
        '-NoProfile',
        '-ExecutionPolicy',
        'Bypass',
        '-WindowStyle',
        'Hidden',
        '-File',
        (Quote-Argument $temporaryScript),
        '-RemovePath',
        (Quote-Argument $installPath),
        '-SkipConfirmation'
    ) -join ' '
    Start-Process -FilePath 'powershell.exe' `
        -ArgumentList $arguments `
        -WorkingDirectory $env:TEMP `
        -WindowStyle Hidden
    exit 0
}

try {
    Get-Process -Name 'spull' -ErrorAction SilentlyContinue | ForEach-Object {
        try {
            if ($_.Path -eq $executable) {
                $_.CloseMainWindow() | Out-Null
                Start-Sleep -Milliseconds 500
                if (-not $_.HasExited) { $_.Kill() }
            }
        } catch {
            # A process that has already exited needs no further action.
        }
    }

    Remove-ShortcutIfTarget -path $shortcutPath -target $executable
    Remove-ShortcutIfTarget `
        -path $uninstallShortcutPath `
        -target $wscript `
        -arguments $uninstaller
    if (Test-Path -LiteralPath $installPath) {
        Remove-Item -LiteralPath $installPath -Recurse -Force
    }

    Show-Message `
        'Spull was uninstalled successfully.' `
        'Spull Uninstall' `
        ([System.Windows.Forms.MessageBoxButtons]::OK) `
        ([System.Windows.Forms.MessageBoxIcon]::Information) | Out-Null
} catch {
    Show-Message `
        "Spull could not be uninstalled.`n`n$($_.Exception.Message)" `
        'Spull Uninstall' `
        ([System.Windows.Forms.MessageBoxButtons]::OK) `
        ([System.Windows.Forms.MessageBoxIcon]::Error) | Out-Null
    exit 1
} finally {
    Remove-Item -LiteralPath $scriptPath -Force -ErrorAction SilentlyContinue
}
