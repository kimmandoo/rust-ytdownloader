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

    Add-Type -AssemblyName System.IO.Compression.FileSystem
    Add-Type -AssemblyName Microsoft.CSharp
    $launcherSource = @'
using System;
using System.Diagnostics;
using System.IO;
using System.IO.Compression;
using System.Reflection;
using System.Runtime.InteropServices;
using System.Text;

internal static class SpullSetupLauncher
{
    private const string PayloadMarker = "SPULL_SETUP_PAYLOAD_V1_7F2C";

    private static int Main()
    {
        try
        {
            var installRoot = Path.Combine(
                Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData),
                "Spull");
            var stagingRoot = installRoot + ".new-" + Guid.NewGuid().ToString("N");
            var temporaryArchive = Path.Combine(
                Path.GetTempPath(),
                "spull-runtime-" + Guid.NewGuid().ToString("N") + ".zip");

            Directory.CreateDirectory(stagingRoot);
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
                    // A locked old install is harmless and can be removed later.
                }
            }

            var executable = Path.Combine(installRoot, "spull.exe");
            TryCreateStartMenuShortcut(executable, installRoot);
            Process.Start(new ProcessStartInfo
            {
                FileName = executable,
                WorkingDirectory = installRoot,
                UseShellExecute = true
            });
            Console.WriteLine("Spull installed to " + installRoot);
            return 0;
        }
        catch (Exception exception)
        {
            Console.Error.WriteLine("Spull setup failed: " + exception.Message);
            return 1;
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
        var payloadLength = bytes.Length - payloadStart;
        var payload = new byte[payloadLength];
        Buffer.BlockCopy(bytes, payloadStart, payload, 0, payloadLength);
        return payload;
    }

    private static void TryCreateStartMenuShortcut(string executable, string installRoot)
    {
        try
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
        catch (Exception exception)
        {
            Console.Error.WriteLine("Start Menu shortcut was not created: " + exception.Message);
        }
    }
}
'@

    Add-Type -TypeDefinition $launcherSource `
        -OutputAssembly $stubExecutable `
        -OutputType ConsoleApplication `
        -ReferencedAssemblies @(
            [System.IO.Compression.ZipFile].Assembly.Location,
            [Microsoft.CSharp.RuntimeBinder.Binder].Assembly.Location
        )

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
