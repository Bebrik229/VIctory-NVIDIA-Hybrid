<#
.SYNOPSIS
    Downloads Microsoft DirectX 12 Agility SDK 1.619.6 (D3D12Core.dll, d3d12SDKLayers.dll)
    into D3D12\ beside victory.exe to match apps\victory\src\main.rs (D3D12SDKVersion = 619).
.DESCRIPTION
    victory.exe exports D3D12SDKVersion = 619 and D3D12SDKPath = ".\D3D12\".
    Without D3D12Core.dll inside that folder, Direct3D 12 rejects startup with
    D3D12_ERROR_INVALID_REDIST (0x887E0003). This script fetches the official NuGet
    Microsoft.Direct3D.D3D12 1.619.6 package using a temporary folder inside the
    target directory (never polluting C:\Users\...\Temp) and extracts only the x64 DLLs.
#>
param(
    [ValidateSet('release', 'diag', 'debug')]
    [string[]]$Build = @('release'),
    [string]$OutDir = ''
)

$ErrorActionPreference = 'Stop'
$Version = '1.619.6'
$Expected = [ordered]@{
    'D3D12Core.dll'      = '37fa14281a58cc834076971873006feb8a8d25cddc908d1a345bda1b149ffc7d'
    'd3d12SDKLayers.dll' = '7ab48d56d1c18708ebcf16c7c9259b2d783e2b935b85add4a244573b9f03cab4'
}
$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path

$Destinations = @()
if ($OutDir) {
    $Destinations += $OutDir
} else {
    foreach ($b in $Build) {
        $binDir = Join-Path $RepoRoot "target\$b"
        if (Test-Path $binDir) {
            $Destinations += (Join-Path $binDir 'D3D12')
        }
    }
    if ($Destinations.Count -eq 0) {
        throw "No build directory found under $RepoRoot\target\$($Build -join ','). Build first with: cargo build --release -j 1 -p victory"
    }
}

$FirstDest = $Destinations[0]
New-Item -ItemType Directory -Path $FirstDest -Force | Out-Null
$WorkDir = Join-Path $FirstDest '.agility-tmp'
New-Item -ItemType Directory -Path $WorkDir -Force | Out-Null

try {
    $PkgZip = Join-Path $WorkDir "microsoft.direct3d.d3d12.$Version.zip"
    $UnpackDir = Join-Path $WorkDir 'unpacked'
    $Url = "https://api.nuget.org/v3-flatcontainer/microsoft.direct3d.d3d12/$Version/microsoft.direct3d.d3d12.$Version.nupkg"

    Write-Host "[get-agility-sdk] Downloading Microsoft.Direct3D.D3D12 $Version from nuget.org..." -ForegroundColor Cyan
    [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
    Invoke-WebRequest -Uri $Url -OutFile $PkgZip -UseBasicParsing

    Expand-Archive -Path $PkgZip -DestinationPath $UnpackDir -Force
    $BinSrc = Join-Path $UnpackDir 'build\native\bin\x64'
    $CoreSrc = Join-Path $BinSrc 'D3D12Core.dll'
    $DebugSrc = Join-Path $BinSrc 'd3d12SDKLayers.dll'
    if (-not (Test-Path $CoreSrc)) {
        throw "Missing D3D12Core.dll inside unpacked NuGet package at $BinSrc"
    }
    foreach ($name in $Expected.Keys) {
        $file = Join-Path $BinSrc $name
        if ((Test-Path $file) -and (Get-FileHash -Algorithm SHA256 -Path $file).Hash.ToLower() -ne $Expected[$name]) {
            throw "$name from NuGet $Version does not match its SHA-256 $($Expected[$name])"
        }
    }

    foreach ($dest in $Destinations) {
        New-Item -ItemType Directory -Path $dest -Force | Out-Null
        Copy-Item -Path $CoreSrc -Destination (Join-Path $dest 'D3D12Core.dll') -Force
        if (Test-Path $DebugSrc) {
            Copy-Item -Path $DebugSrc -Destination (Join-Path $dest 'd3d12SDKLayers.dll') -Force
        }
        Write-Host "[get-agility-sdk] Installed D3D12Core.dll ($Version) -> $((Resolve-Path $dest).Path)" -ForegroundColor Green
    }
} finally {
    if (Test-Path $WorkDir) {
        Remove-Item -Path $WorkDir -Recurse -Force -ErrorAction SilentlyContinue
    }
}
