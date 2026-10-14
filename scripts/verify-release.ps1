#requires -Version 7.0
param(
    [Parameter(Mandatory)][string]$Executable,
    [Parameter(Mandatory)][string]$EvidenceDirectory
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$exe = (Resolve-Path -LiteralPath $Executable).Path
$evidence = [IO.Path]::GetFullPath($EvidenceDirectory)
if (Test-Path -LiteralPath $evidence) { throw 'Use a fresh evidence directory.' }
New-Item -ItemType Directory -Path $evidence | Out-Null
$repoRoot = Split-Path -Parent $PSScriptRoot

Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
using System.Text;
public static class CalamariReleaseCheck {
    [DllImport("kernel32.dll", CharSet=CharSet.Unicode, SetLastError=true)]
    static extern IntPtr LoadLibraryEx(string path, IntPtr file, uint flags);
    [DllImport("kernel32.dll", CharSet=CharSet.Unicode, SetLastError=true)]
    static extern IntPtr FindResource(IntPtr module, IntPtr name, IntPtr type);
    [DllImport("kernel32.dll")] static extern uint SizeofResource(IntPtr module, IntPtr resource);
    [DllImport("kernel32.dll")] static extern IntPtr LoadResource(IntPtr module, IntPtr resource);
    [DllImport("kernel32.dll")] static extern IntPtr LockResource(IntPtr resource);
    [DllImport("kernel32.dll")] static extern bool FreeLibrary(IntPtr module);
    public static byte[] Resource(string path, int id, int type) {
        var module = LoadLibraryEx(path, IntPtr.Zero, 2);
        if (module == IntPtr.Zero) throw new Exception("Cannot load executable resources");
        try {
            var resource = FindResource(module, (IntPtr)id, (IntPtr)type);
            if (resource == IntPtr.Zero) throw new Exception("Missing resource " + type + "/" + id);
            var bytes = new byte[SizeofResource(module, resource)];
            Marshal.Copy(LockResource(LoadResource(module, resource)), bytes, 0, bytes.Length);
            return bytes;
        } finally { FreeLibrary(module); }
    }
    delegate bool WindowCallback(IntPtr window, IntPtr data);
    [DllImport("user32.dll")] static extern bool EnumWindows(WindowCallback callback, IntPtr data);
    [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr window, out uint process);
    [DllImport("user32.dll", CharSet=CharSet.Unicode)] static extern int GetWindowText(IntPtr window, StringBuilder text, int count);
    [DllImport("user32.dll")] static extern IntPtr SendMessageTimeout(IntPtr window, uint msg, UIntPtr wparam, IntPtr lparam, uint flags, uint timeout, out UIntPtr result);
    [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr window, uint msg, IntPtr wparam, IntPtr lparam);
    public static IntPtr FindWindow(uint process) {
        IntPtr found = IntPtr.Zero;
        EnumWindows((window, data) => {
            uint owner; GetWindowThreadProcessId(window, out owner);
            var title = new StringBuilder(256); GetWindowText(window, title, 256);
            if (owner == process && title.ToString() == "Rusted Calamari") { found = window; return false; }
            return true;
        }, IntPtr.Zero);
        return found;
    }
    public static bool HasWindowIcon(IntPtr window) {
        UIntPtr icon;
        return SendMessageTimeout(window, 0x7f, (UIntPtr)1, IntPtr.Zero, 2, 1000, out icon) != IntPtr.Zero && icon != UIntPtr.Zero;
    }
}
'@

$bytes = [IO.File]::ReadAllBytes($exe)
$pe = [BitConverter]::ToInt32($bytes, 0x3c)
if ([BitConverter]::ToUInt16($bytes, $pe + 4) -ne 0x8664) { throw 'Expected a Windows x64 executable.' }
if ([BitConverter]::ToUInt16($bytes, $pe + 24 + 68) -ne 2) { throw 'Expected the Windows GUI subsystem.' }
$version = [Diagnostics.FileVersionInfo]::GetVersionInfo($exe)
if ($version.ProductName -ne 'Rusted Calamari') { throw 'Missing application version resource.' }

# Compare every embedded RT_ICON with the corresponding source ICO payload.
$ico = [IO.File]::ReadAllBytes((Join-Path $repoRoot 'assets/icons/rusted-calamari.ico'))
$group = [CalamariReleaseCheck]::Resource($exe, 1, 14)
$count = [BitConverter]::ToUInt16($group, 4)
if ($count -ne 9 -or $count -ne [BitConverter]::ToUInt16($ico, 4)) { throw 'Missing icon resolutions.' }
$sizes = @()
for ($i = 0; $i -lt $count; $i++) {
    $entry = 6 + 16 * $i
    $size = [BitConverter]::ToInt32($ico, $entry + 8)
    $offset = [BitConverter]::ToInt32($ico, $entry + 12)
    $id = [BitConverter]::ToUInt16($group, 6 + 14 * $i + 12)
    $embedded = [CalamariReleaseCheck]::Resource($exe, $id, 3)
    $expected = [byte[]]$ico[$offset..($offset + $size - 1)]
    if ([Convert]::ToBase64String($embedded) -cne [Convert]::ToBase64String($expected)) {
        throw "Embedded icon frame $i differs from the supplied artwork."
    }
    $sizes += $(if ($ico[$entry] -eq 0) { 256 } else { [int]$ico[$entry] })
}

# Launch the actual distributable with its own profile and a tiny fixture library.
$fixture = Join-Path $evidence 'fixture'
New-Item -ItemType Directory -Path $fixture | Out-Null
Copy-Item -LiteralPath (Join-Path $repoRoot 'assets/icons/rusted-calamari.png') -Destination (Join-Path $fixture 'sample.png')
$start = [Diagnostics.ProcessStartInfo]::new($exe)
$start.UseShellExecute = $false
$start.WindowStyle = [Diagnostics.ProcessWindowStyle]::Hidden
$start.WorkingDirectory = Split-Path -Parent $exe
$start.ArgumentList.Add($fixture)
$start.Environment['APPDATA'] = Join-Path $evidence 'profile/Roaming'
$start.Environment['LOCALAPPDATA'] = Join-Path $evidence 'profile/Local'
$process = [Diagnostics.Process]::Start($start)
try {
    $window = [IntPtr]::Zero
    for ($attempt = 0; $attempt -lt 100; $attempt++) {
        if ($process.HasExited) { throw "Application exited early: $($process.ExitCode)" }
        $window = [CalamariReleaseCheck]::FindWindow([uint32]$process.Id)
        if ($window -ne [IntPtr]::Zero -and [CalamariReleaseCheck]::HasWindowIcon($window)) { break }
        Start-Sleep -Milliseconds 100
    }
    if ($window -eq [IntPtr]::Zero -or -not [CalamariReleaseCheck]::HasWindowIcon($window)) {
        throw 'Application did not expose its native window and runtime icon.'
    }
    if (-not [CalamariReleaseCheck]::PostMessage($window, 0x10, [IntPtr]::Zero, [IntPtr]::Zero)) {
        throw 'Could not request normal window closure.'
    }
    if (-not $process.WaitForExit(10000)) { throw 'Application did not close normally.' }
    if ($process.ExitCode -ne 0) { throw "Application exited with $($process.ExitCode)." }
    [ordered]@{
        result = 'PASS'
        executable_sha256 = (Get-FileHash -LiteralPath $exe -Algorithm SHA256).Hash.ToLowerInvariant()
        architecture = 'x86_64'
        subsystem = 'Windows GUI'
        product_version = $version.ProductVersion
        icon_sizes = $sizes
        embedded_icon_payloads_match = $true
        native_window_and_runtime_icon = $true
        normal_exit_code = $process.ExitCode
    } | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $evidence 'RESULT.json') -Encoding utf8
    Get-Content -LiteralPath (Join-Path $evidence 'RESULT.json')
}
finally {
    if (-not $process.HasExited) { $process.Kill(); $process.WaitForExit() }
    $process.Dispose()
}
