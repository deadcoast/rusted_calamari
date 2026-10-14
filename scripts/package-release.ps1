#requires -Version 7.0
<#
.SYNOPSIS
Build a Windows portable ZIP and SHA-256 checksum from this checkout.
Generated packages stay under ignored target/ by default. Existing packages
are never overwritten; pass a different OutputDirectory for another run.
#>
param([string]$OutputDirectory)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$repoRoot = Split-Path -Parent $PSScriptRoot
if (-not $IsWindows) { throw 'Run this packaging script on Windows.' }
if (-not $OutputDirectory) { $OutputDirectory = Join-Path $repoRoot 'target/packages' }
$outputRoot = [IO.Path]::GetFullPath($OutputDirectory)

Push-Location $repoRoot
try {
    $hostTriple = ((rustc -vV | Select-String '^host: ').Line -replace '^host: ', '').Trim()
    if ($LASTEXITCODE -ne 0 -or $hostTriple -notmatch '-pc-windows-msvc$') {
        throw 'The Windows MSVC Rust toolchain is required.'
    }
    # The explicit host target makes the archive architecture unambiguous.
    cargo build --release --locked --target $hostTriple --bin rusted-calamari
    if ($LASTEXITCODE -ne 0) { throw 'Release build failed.' }
    $metadataText = cargo metadata --locked --offline --filter-platform $hostTriple --format-version 1
    if ($LASTEXITCODE -ne 0) { throw 'Could not resolve package metadata.' }
    $metadata = $metadataText | ConvertFrom-Json
    $package = $metadata.packages | Where-Object { $_.name -eq 'rusted-calamari' }
    $name = "rusted-calamari-$($package.version)-$hostTriple"
    $stage = Join-Path $outputRoot $name
    $archive = Join-Path $outputRoot "$name.zip"
    foreach ($path in @($stage, $archive, "$archive.sha256")) {
        if (Test-Path -LiteralPath $path) { throw "Output already exists: $path" }
    }
    New-Item -ItemType Directory -Path $stage -Force | Out-Null
    $exe = Join-Path $metadata.target_directory "$hostTriple/release/rusted-calamari.exe"
    Copy-Item -LiteralPath $exe -Destination $stage
    foreach ($doc in @('README.md', 'DESIGN.md', 'UX-CONTRACT.md', 'OCTOBRACHIAL_TREE_FORMAT.md')) {
        Copy-Item -LiteralPath (Join-Path $repoRoot $doc) -Destination $stage
    }
    Copy-Item -LiteralPath (Join-Path $repoRoot 'docs') -Destination $stage -Recurse
    $fontDestination = Join-Path $stage 'assets/fonts'
    New-Item -ItemType Directory -Path $fontDestination -Force | Out-Null
    Copy-Item -LiteralPath (Join-Path $repoRoot 'assets/fonts/LICENSE.TXT') -Destination $fontDestination
    Copy-Item -LiteralPath (Join-Path $repoRoot 'assets/icons') -Destination (Join-Path $stage 'assets') -Recurse
    Copy-Item -LiteralPath (Join-Path $repoRoot 'assets/logos') -Destination (Join-Path $stage 'assets') -Recurse

    # Preserve the dependency declarations and shipped upstream licence/notice files.
    $licenses = Join-Path $stage 'licenses'
    New-Item -ItemType Directory -Path $licenses -Force | Out-Null
    $inventory = foreach ($dependency in ($metadata.packages | Where-Object source | Sort-Object name, version)) {
        $dependencyRoot = Split-Path -Parent $dependency.manifest_path
        $dest = Join-Path $licenses "$($dependency.name)-$($dependency.version)"
        $notices = @(Get-ChildItem -LiteralPath $dependencyRoot | Where-Object {
            $_.Name -match '^(LICENSE|LICENCE|COPYING|NOTICE|COPYRIGHT|AUTHORS)([.\-_]|$)'
        })
        if ($notices.Count -gt 0) {
            New-Item -ItemType Directory -Path $dest -Force | Out-Null
            foreach ($notice in $notices) { Copy-Item -LiteralPath $notice.FullName -Destination $dest -Recurse }
        }
        if ($dependency.license_file) {
            $licenseFile = Join-Path $dependencyRoot $dependency.license_file
            if (Test-Path -LiteralPath $licenseFile -PathType Leaf) {
                New-Item -ItemType Directory -Path $dest -Force | Out-Null
                Copy-Item -LiteralPath $licenseFile -Destination $dest -Force
            }
        }
        "$($dependency.name) $($dependency.version)`t$($dependency.license)`t$($dependency.repository)"
    }
    $inventory | Set-Content -LiteralPath (Join-Path $licenses 'DEPENDENCIES.txt') -Encoding utf8
    @"
Rusted Calamari $($package.version) - Windows portable application

Extract the whole ZIP, then double-click rusted-calamari.exe.
No Rust toolchain or installer is needed to run it.
Open a folder with Ctrl+O or drop a folder onto the window.
The app writes preferences under your Windows user profile.
Sorting keys move real files; Ctrl+Z undoes within the current library session.

See README.md for controls and the two-tree workspace.
Source and updates: https://github.com/deadcoast/rusted_calamari
Font attribution: VileR, The Ultimate Oldschool PC Font Pack, CC BY-SA 4.0.
Font licence: assets/fonts/LICENSE.TXT. Crate notices: licenses/.
"@ | Set-Content -LiteralPath (Join-Path $stage 'START-HERE.txt') -Encoding utf8
    $buildInfo = [ordered]@{
        version = $package.version
        target = $hostTriple
        source_commit = (git rev-parse HEAD).Trim()
        source_dirty = [bool](git status --porcelain)
        rustc = (rustc --version)
        executable_sha256 = (Get-FileHash -LiteralPath $exe -Algorithm SHA256).Hash.ToLowerInvariant()
    }
    $buildInfo | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $stage 'BUILD-INFO.json') -Encoding utf8
    Compress-Archive -LiteralPath $stage -DestinationPath $archive -CompressionLevel Optimal
    $hash = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant()
    "$hash  $name.zip" | Set-Content -LiteralPath "$archive.sha256" -Encoding ascii
    Write-Output "Package: $archive"
    Write-Output "SHA-256: $hash"
}
finally { Pop-Location }
