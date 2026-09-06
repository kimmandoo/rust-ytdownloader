$ErrorActionPreference = 'Stop'

Add-Type -AssemblyName System.Drawing
Add-Type -AssemblyName System.Windows.Forms

$scriptRoot = Split-Path -Parent $MyInvocation.MyCommand.Definition
$payloadArchive = Join-Path $scriptRoot 'spull-runtime.zip'
$uninstallScript = Join-Path $scriptRoot 'Uninstall-Spull.ps1'
$uninstallLauncher = Join-Path $scriptRoot 'Uninstall-Spull.vbs'

function Show-Error([string]$message) {
    [System.Windows.Forms.MessageBox]::Show(
        $message,
        'Spull Setup',
        [System.Windows.Forms.MessageBoxButtons]::OK,
        [System.Windows.Forms.MessageBoxIcon]::Error) | Out-Null
}

if (-not (Test-Path -LiteralPath $payloadArchive -PathType Leaf)) {
    Show-Error "The Spull runtime archive is missing from $scriptRoot."
    exit 1
}
if (-not (Test-Path -LiteralPath $uninstallScript -PathType Leaf) -or
    -not (Test-Path -LiteralPath $uninstallLauncher -PathType Leaf)) {
    Show-Error "The Spull uninstaller is missing from $scriptRoot."
    exit 1
}

$form = New-Object System.Windows.Forms.Form
$form.Text = 'Spull Setup'
$form.ClientSize = New-Object System.Drawing.Size(560, 330)
$form.FormBorderStyle = [System.Windows.Forms.FormBorderStyle]::FixedDialog
$form.MaximizeBox = $false
$form.MinimizeBox = $false
$form.StartPosition = [System.Windows.Forms.FormStartPosition]::CenterScreen
$form.BackColor = [System.Drawing.Color]::White

$heading = New-Object System.Windows.Forms.Label
$heading.AutoSize = $true
$heading.Location = New-Object System.Drawing.Point(28, 24)
$heading.Text = 'Install Spull'
$heading.Font = New-Object System.Drawing.Font('Segoe UI', 17, [System.Drawing.FontStyle]::Bold)
$form.Controls.Add($heading)

$description = New-Object System.Windows.Forms.Label
$description.AutoSize = $true
$description.Location = New-Object System.Drawing.Point(30, 62)
$description.Text = 'Choose where Spull should be installed, then click Install.'
$form.Controls.Add($description)

$locationLabel = New-Object System.Windows.Forms.Label
$locationLabel.AutoSize = $true
$locationLabel.Location = New-Object System.Drawing.Point(30, 108)
$locationLabel.Text = 'Installation folder:'
$form.Controls.Add($locationLabel)

$installPath = New-Object System.Windows.Forms.TextBox
$installPath.Location = New-Object System.Drawing.Point(30, 132)
$installPath.Size = New-Object System.Drawing.Size(402, 24)
$installPath.Text = Join-Path ([Environment]::GetFolderPath('LocalApplicationData')) 'Spull'
$form.Controls.Add($installPath)

$browseButton = New-Object System.Windows.Forms.Button
$browseButton.Location = New-Object System.Drawing.Point(442, 131)
$browseButton.Size = New-Object System.Drawing.Size(88, 26)
$browseButton.Text = 'Browse...'
$form.Controls.Add($browseButton)

$createShortcut = New-Object System.Windows.Forms.CheckBox
$createShortcut.AutoSize = $true
$createShortcut.Location = New-Object System.Drawing.Point(30, 180)
$createShortcut.Checked = $true
$createShortcut.Text = 'Create a Start Menu shortcut'
$form.Controls.Add($createShortcut)

$launchAfterInstall = New-Object System.Windows.Forms.CheckBox
$launchAfterInstall.AutoSize = $true
$launchAfterInstall.Location = New-Object System.Drawing.Point(30, 207)
$launchAfterInstall.Checked = $true
$launchAfterInstall.Text = 'Launch Spull after installation'
$form.Controls.Add($launchAfterInstall)

$statusLabel = New-Object System.Windows.Forms.Label
$statusLabel.AutoSize = $true
$statusLabel.Location = New-Object System.Drawing.Point(30, 239)
$statusLabel.Text = 'Ready to install.'
$form.Controls.Add($statusLabel)

$progressBar = New-Object System.Windows.Forms.ProgressBar
$progressBar.Location = New-Object System.Drawing.Point(30, 262)
$progressBar.Size = New-Object System.Drawing.Size(500, 18)
$form.Controls.Add($progressBar)

$installButton = New-Object System.Windows.Forms.Button
$installButton.Location = New-Object System.Drawing.Point(350, 294)
$installButton.Size = New-Object System.Drawing.Size(86, 28)
$installButton.Text = 'Install'
$form.Controls.Add($installButton)

$cancelButton = New-Object System.Windows.Forms.Button
$cancelButton.DialogResult = [System.Windows.Forms.DialogResult]::Cancel
$cancelButton.Location = New-Object System.Drawing.Point(444, 294)
$cancelButton.Size = New-Object System.Drawing.Size(86, 28)
$cancelButton.Text = 'Cancel'
$form.Controls.Add($cancelButton)
$form.AcceptButton = $installButton
$form.CancelButton = $cancelButton

$browseButton.Add_Click({
    $dialog = New-Object System.Windows.Forms.FolderBrowserDialog
    try {
        $dialog.Description = 'Choose the Spull installation folder'
        $dialog.SelectedPath = $installPath.Text
        if ($dialog.ShowDialog($form) -eq [System.Windows.Forms.DialogResult]::OK) {
            $installPath.Text = Join-Path $dialog.SelectedPath 'Spull'
        }
    }
    finally {
        $dialog.Dispose()
    }
})

$installButton.Add_Click({
    $destination = $installPath.Text.Trim()
    if ([string]::IsNullOrWhiteSpace($destination)) {
        [System.Windows.Forms.MessageBox]::Show(
            $form,
            'Choose an installation folder first.',
            'Spull Setup',
            [System.Windows.Forms.MessageBoxButtons]::OK,
            [System.Windows.Forms.MessageBoxIcon]::Warning) | Out-Null
        return
    }
    if (-not [System.IO.Path]::IsPathRooted($destination)) {
        [System.Windows.Forms.MessageBox]::Show(
            $form,
            'The installation folder must be an absolute path.',
            'Spull Setup',
            [System.Windows.Forms.MessageBoxButtons]::OK,
            [System.Windows.Forms.MessageBoxIcon]::Warning) | Out-Null
        return
    }

    $installButton.Enabled = $false
    $cancelButton.Enabled = $false
    $statusLabel.Text = 'Installing Spull...'
    $progressBar.Style = [System.Windows.Forms.ProgressBarStyle]::Marquee
    $form.Refresh()

    $staging = "$destination.new-$([guid]::NewGuid().ToString('N'))"
    $backup = $null
    $temporaryArchive = Join-Path $env:TEMP "spull-runtime-$([guid]::NewGuid().ToString('N')).zip"
    try {
        New-Item -ItemType Directory -Path $staging -Force | Out-Null
        Copy-Item -LiteralPath $payloadArchive -Destination $temporaryArchive -Force
        Expand-Archive -LiteralPath $temporaryArchive -DestinationPath $staging -Force
        Copy-Item -LiteralPath $uninstallScript -Destination $staging -Force
        Copy-Item -LiteralPath $uninstallLauncher -Destination $staging -Force

        if (Test-Path -LiteralPath $destination) {
            $backup = "$destination.old-$([guid]::NewGuid().ToString('N'))"
            Move-Item -LiteralPath $destination -Destination $backup
        }
        Move-Item -LiteralPath $staging -Destination $destination
        if ($backup -and (Test-Path -LiteralPath $backup)) {
            Remove-Item -LiteralPath $backup -Recurse -Force -ErrorAction SilentlyContinue
        }

        $executable = Join-Path $destination 'spull.exe'
        $uninstaller = Join-Path $destination 'Uninstall-Spull.vbs'
        if ($createShortcut.Checked) {
            $programs = Join-Path ([Environment]::GetFolderPath('ApplicationData')) 'Microsoft\Windows\Start Menu\Programs'
            New-Item -ItemType Directory -Path $programs -Force | Out-Null
            $shell = New-Object -ComObject WScript.Shell

            $shortcutPath = Join-Path $programs 'Spull.lnk'
            $shortcut = $shell.CreateShortcut($shortcutPath)
            $shortcut.TargetPath = $executable
            $shortcut.WorkingDirectory = $destination
            $shortcut.Description = 'Spull media downloader'
            $shortcut.Save()

            $uninstallShortcutPath = Join-Path $programs 'Uninstall Spull.lnk'
            $uninstallShortcut = $shell.CreateShortcut($uninstallShortcutPath)
            $uninstallShortcut.TargetPath = Join-Path $env:WINDIR 'System32\wscript.exe'
            $uninstallShortcut.Arguments = '"' + $uninstaller + '"'
            $uninstallShortcut.WorkingDirectory = $destination
            $uninstallShortcut.Description = 'Uninstall Spull'
            $uninstallShortcut.Save()
        }
        if ($launchAfterInstall.Checked) {
            Start-Process -FilePath $executable -WorkingDirectory $destination
        }

        $progressBar.Style = [System.Windows.Forms.ProgressBarStyle]::Continuous
        $progressBar.Value = 100
        [System.Windows.Forms.MessageBox]::Show(
            $form,
            'Spull was installed successfully.',
            'Setup complete',
            [System.Windows.Forms.MessageBoxButtons]::OK,
            [System.Windows.Forms.MessageBoxIcon]::Information) | Out-Null
        $form.Close()
    }
    catch {
        if ($backup -and (Test-Path -LiteralPath $backup) -and -not (Test-Path -LiteralPath $destination)) {
            Move-Item -LiteralPath $backup -Destination $destination -ErrorAction SilentlyContinue
        }
        $progressBar.Style = [System.Windows.Forms.ProgressBarStyle]::Continuous
        $progressBar.Value = 0
        $installButton.Enabled = $true
        $cancelButton.Enabled = $true
        $statusLabel.Text = 'Installation failed.'
        Show-Error $_.Exception.Message
    }
    finally {
        Remove-Item -LiteralPath $temporaryArchive -Force -ErrorAction SilentlyContinue
        Remove-Item -LiteralPath $staging -Recurse -Force -ErrorAction SilentlyContinue
    }
})

[void]$form.ShowDialog()
$form.Dispose()
