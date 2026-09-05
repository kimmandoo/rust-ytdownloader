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

$outputPath = [System.IO.Path]::GetFullPath($OutputDirectory)
New-Item -ItemType Directory -Path $outputPath -Force | Out-Null

$portableArchive = Join-Path $outputPath "spull-windows-x86_64.zip"
$setupExecutable = Join-Path $outputPath "spull-windows-x86_64-setup.exe"
$temporaryDirectory = Join-Path ([System.IO.Path]::GetTempPath()) "spull-package-$PID"
$payloadArchive = Join-Path $temporaryDirectory "spull-runtime.zip"
$stubExecutable = Join-Path $temporaryDirectory "spull-setup.stub.exe"

try {
    New-Item -ItemType Directory -Path $temporaryDirectory -Force | Out-Null

    Compress-Archive -Path (Join-Path $releasePath "*") `
        -DestinationPath $portableArchive -CompressionLevel Optimal -Force
    Compress-Archive -Path (Join-Path $releasePath "*") `
        -DestinationPath $payloadArchive -CompressionLevel Optimal -Force

    $launcherSource = @'
using System;
using System.Diagnostics;
using System.Drawing;
using System.IO;
using System.IO.Compression;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Text;
using System.Windows.Forms;

internal static class SpullSetupLauncher
{
    private const string PayloadMarker = "SPULL_SETUP_PAYLOAD_V1_7F2C";

    [STAThread]
    private static void Main()
    {
        Application.EnableVisualStyles();
        Application.SetCompatibleTextRenderingDefault(false);
        Application.Run(new SetupForm());
    }

    private sealed class SetupForm : Form
    {
        private readonly TextBox installPath;
        private readonly CheckBox createShortcut;
        private readonly CheckBox launchAfterInstall;
        private readonly Button installButton;
        private readonly Button cancelButton;
        private readonly Label statusLabel;
        private readonly ProgressBar progressBar;

        public SetupForm()
        {
            Text = "Spull Setup";
            ClientSize = new Size(560, 330);
            FormBorderStyle = FormBorderStyle.FixedDialog;
            MaximizeBox = false;
            MinimizeBox = false;
            StartPosition = FormStartPosition.CenterScreen;
            BackColor = Color.White;

            var heading = new Label
            {
                AutoSize = true,
                Location = new Point(28, 24),
                Text = "Install Spull",
                Font = new Font("Segoe UI", 17, FontStyle.Bold)
            };
            Controls.Add(heading);

            var description = new Label
            {
                AutoSize = true,
                Location = new Point(30, 62),
                MaximumSize = new Size(500, 0),
                Text = "Choose where Spull should be installed, then click Install."
            };
            Controls.Add(description);

            var locationLabel = new Label
            {
                AutoSize = true,
                Location = new Point(30, 108),
                Text = "Installation folder:"
            };
            Controls.Add(locationLabel);

            installPath = new TextBox
            {
                Location = new Point(30, 132),
                Size = new Size(402, 24),
                Text = Path.Combine(
                    Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
                    "Spull")
            };
            Controls.Add(installPath);

            var browseButton = new Button
            {
                Location = new Point(442, 131),
                Size = new Size(88, 26),
                Text = "Browse..."
            };
            browseButton.Click += BrowseButton_Click;
            Controls.Add(browseButton);

            createShortcut = new CheckBox
            {
                AutoSize = true,
                Location = new Point(30, 180),
                Checked = true,
                Text = "Create a Start Menu shortcut"
            };
            Controls.Add(createShortcut);

            launchAfterInstall = new CheckBox
            {
                AutoSize = true,
                Location = new Point(30, 207),
                Checked = true,
                Text = "Launch Spull after installation"
            };
            Controls.Add(launchAfterInstall);

            statusLabel = new Label
            {
                AutoSize = true,
                Location = new Point(30, 239),
                Text = "Ready to install."
            };
            Controls.Add(statusLabel);

            progressBar = new ProgressBar
            {
                Location = new Point(30, 262),
                Size = new Size(500, 18),
                Style = ProgressBarStyle.Continuous
            };
            Controls.Add(progressBar);

            installButton = new Button
            {
                DialogResult = DialogResult.None,
                Location = new Point(350, 294),
                Size = new Size(86, 28),
                Text = "Install"
            };
            installButton.Click += InstallButton_Click;
            Controls.Add(installButton);

            cancelButton = new Button
            {
                DialogResult = DialogResult.Cancel,
                Location = new Point(444, 294),
                Size = new Size(86, 28),
                Text = "Cancel"
            };
            cancelButton.Click += delegate { Close(); };
            Controls.Add(cancelButton);

            AcceptButton = installButton;
            CancelButton = cancelButton;
        }

        private void BrowseButton_Click(object sender, EventArgs args)
        {
            using (var dialog = new FolderBrowserDialog())
            {
                dialog.Description = "Choose the Spull installation folder";
                dialog.SelectedPath = installPath.Text;
                if (dialog.ShowDialog(this) == DialogResult.OK)
                {
                    installPath.Text = Path.Combine(dialog.SelectedPath, "Spull");
                }
            }
        }

        private void InstallButton_Click(object sender, EventArgs args)
        {
            var destination = installPath.Text.Trim();
            if (string.IsNullOrWhiteSpace(destination))
            {
                MessageBox.Show(
                    this,
                    "Choose an installation folder first.",
                    "Spull Setup",
                    MessageBoxButtons.OK,
                    MessageBoxIcon.Warning);
                return;
            }
            if (!Path.IsPathRooted(destination))
            {
                MessageBox.Show(
                    this,
                    "The installation folder must be an absolute path.",
                    "Spull Setup",
                    MessageBoxButtons.OK,
                    MessageBoxIcon.Warning);
                return;
            }

            installButton.Enabled = false;
            cancelButton.Enabled = false;
            statusLabel.Text = "Installing Spull...";
            progressBar.Style = ProgressBarStyle.Marquee;
            Refresh();

            try
            {
                Install(destination);
                progressBar.Style = ProgressBarStyle.Continuous;
                progressBar.Value = 100;
                MessageBox.Show(
                    this,
                    "Spull was installed successfully.",
                    "Setup complete",
                    MessageBoxButtons.OK,
                    MessageBoxIcon.Information);
                Close();
            }
            catch (Exception exception)
            {
                progressBar.Style = ProgressBarStyle.Continuous;
                progressBar.Value = 0;
                installButton.Enabled = true;
                cancelButton.Enabled = true;
                statusLabel.Text = "Installation failed.";
                MessageBox.Show(
                    this,
                    exception.Message,
                    "Spull Setup",
                    MessageBoxButtons.OK,
                    MessageBoxIcon.Error);
            }
        }

        private void Install(string installRoot)
        {
            var stagingRoot = installRoot + ".new-" + Guid.NewGuid().ToString("N");
            var temporaryArchive = Path.Combine(
                Path.GetTempPath(),
                "spull-runtime-" + Guid.NewGuid().ToString("N") + ".zip");
            Directory.CreateDirectory(stagingRoot);
            try
            {
                File.WriteAllBytes(temporaryArchive, ReadPayload());
                ZipFile.ExtractToDirectory(temporaryArchive, stagingRoot);
                File.Delete(temporaryArchive);

                string backupRoot = null;
                if (Directory.Exists(installRoot))
                {
                    backupRoot = installRoot + ".old-" + Guid.NewGuid().ToString("N");
                    Directory.Move(installRoot, backupRoot);
                }
                Directory.Move(stagingRoot, installRoot);
                if (backupRoot != null)
                {
                    try
                    {
                        Directory.Delete(backupRoot, true);
                    }
                    catch
                    {
                        // A locked old install can be removed later.
                    }
                }

                var executable = Path.Combine(installRoot, "spull.exe");
                if (createShortcut.Checked)
                {
                    CreateStartMenuShortcut(executable, installRoot);
                }
                if (launchAfterInstall.Checked)
                {
                    Process.Start(new ProcessStartInfo
                    {
                        FileName = executable,
                        WorkingDirectory = installRoot,
                        UseShellExecute = true
                    });
                }
            }
            finally
            {
                if (File.Exists(temporaryArchive))
                {
                    File.Delete(temporaryArchive);
                }
                if (Directory.Exists(stagingRoot))
                {
                    Directory.Delete(stagingRoot, true);
                }
            }
        }

        private static byte[] ReadPayload()
        {
            var executable = Assembly.GetExecutingAssembly().Location;
            var bytes = File.ReadAllBytes(executable);
            var marker = Encoding.ASCII.GetBytes(PayloadMarker);
            var markerIndex = -1;
            for (var index = bytes.Length - marker.Length; index >= 0; index--)
            {
                var matches = true;
                for (var markerOffset = 0; markerOffset < marker.Length; markerOffset++)
                {
                    if (bytes[index + markerOffset] != marker[markerOffset])
                    {
                        matches = false;
                        break;
                    }
                }
                if (matches)
                {
                    markerIndex = index;
                    break;
                }
            }
            if (markerIndex < 0)
            {
                throw new InvalidDataException("The embedded Spull runtime was not found.");
            }

            var payloadStart = markerIndex + marker.Length;
            var payload = new byte[bytes.Length - payloadStart];
            Buffer.BlockCopy(bytes, payloadStart, payload, 0, payload.Length);
            return payload;
        }

        private static void CreateStartMenuShortcut(string executable, string installRoot)
        {
            var programs = Path.Combine(
                Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData),
                "Microsoft\\Windows\\Start Menu\\Programs");
            Directory.CreateDirectory(programs);
            var shortcutPath = Path.Combine(programs, "Spull.lnk");
            var shellType = Type.GetTypeFromProgID("WScript.Shell");
            dynamic shell = Activator.CreateInstance(shellType);
            dynamic shortcut = shell.CreateShortcut(shortcutPath);
            shortcut.TargetPath = executable;
            shortcut.WorkingDirectory = installRoot;
            shortcut.Description = "Spull media downloader";
            shortcut.Save();
            Marshal.FinalReleaseComObject(shortcut);
            Marshal.FinalReleaseComObject(shell);
        }
    }
}
'@

    $launcherSourcePath = Join-Path $temporaryDirectory "SpullSetupLauncher.cs"
    Set-Content -Path $launcherSourcePath -Value $launcherSource -Encoding UTF8

    $compiler = @(
        Join-Path $env:WINDIR "Microsoft.NET\Framework64\v4.0.30319\csc.exe"
        Join-Path $env:WINDIR "Microsoft.NET\Framework\v4.0.30319\csc.exe"
    ) | Where-Object { Test-Path $_ } | Select-Object -First 1
    if ([string]::IsNullOrWhiteSpace($compiler)) {
        throw "The .NET Framework C# compiler was not found."
    }

    $frameworkDirectory = Split-Path -Parent $compiler
    $compilerArguments = @(
        "/nologo"
        "/target:winexe"
        "/platform:x64"
        "/optimize+"
        "/out:$stubExecutable"
        "/reference:$(Join-Path $frameworkDirectory 'System.dll')"
        "/reference:$(Join-Path $frameworkDirectory 'System.Core.dll')"
        "/reference:$(Join-Path $frameworkDirectory 'System.Drawing.dll')"
        "/reference:$(Join-Path $frameworkDirectory 'System.IO.Compression.dll')"
        "/reference:$(Join-Path $frameworkDirectory 'System.IO.Compression.FileSystem.dll')"
        "/reference:$(Join-Path $frameworkDirectory 'System.Windows.Forms.dll')"
        "/reference:$(Join-Path $frameworkDirectory 'Microsoft.CSharp.dll')"
        $launcherSourcePath
    )
    & $compiler @compilerArguments
    if ($LASTEXITCODE -ne 0) {
        throw "The setup launcher compilation failed with exit code $LASTEXITCODE."
    }

    $stubBytes = [System.IO.File]::ReadAllBytes($stubExecutable)
    $markerBytes = [System.Text.Encoding]::ASCII.GetBytes("SPULL_SETUP_PAYLOAD_V1_7F2C")
    $payloadBytes = [System.IO.File]::ReadAllBytes($payloadArchive)
    $outputStream = [System.IO.File]::Open(
        $setupExecutable,
        [System.IO.FileMode]::Create,
        [System.IO.FileAccess]::Write,
        [System.IO.FileShare]::None)
    try {
        $outputStream.Write($stubBytes, 0, $stubBytes.Length)
        $outputStream.Write($markerBytes, 0, $markerBytes.Length)
        $outputStream.Write($payloadBytes, 0, $payloadBytes.Length)
    }
    finally {
        $outputStream.Dispose()
    }

    Write-Output "Created $portableArchive"
    Write-Output "Created $setupExecutable"
}
finally {
    if (Test-Path $temporaryDirectory) {
        Remove-Item -Recurse -Force $temporaryDirectory
    }
}
