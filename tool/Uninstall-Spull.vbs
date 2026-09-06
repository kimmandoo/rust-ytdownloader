Option Explicit

Dim shell, files, root, scriptPath, command
Set shell = CreateObject("WScript.Shell")
Set files = CreateObject("Scripting.FileSystemObject")
root = files.GetParentFolderName(WScript.ScriptFullName)
scriptPath = files.BuildPath(root, "Uninstall-Spull.ps1")

If Not files.FileExists(scriptPath) Then
    MsgBox "Uninstall-Spull.ps1 is missing from the Spull installation.", 16, "Spull Uninstall"
    WScript.Quit 1
End If

command = "powershell.exe -NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -File " & Quote(scriptPath)
shell.Run command, 0, False

Function Quote(value)
    Quote = Chr(34) & value & Chr(34)
End Function
