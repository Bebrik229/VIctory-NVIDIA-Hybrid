<#
.SYNOPSIS
    Launches VIctory-NVIDIA-Hybrid with NVIDIA 12 GB + Native Barriers + Multi-Core Ports 1/6/7 (PORT_CBV=0 NVIDIA color fix).
.DESCRIPTION
    Portable launcher derived from run-cgbj-fast.ps1.
    Supports optional overrides while defaulting to local D:\gta6-re\... paths when present.
#>
param(
    [string]$RepoRoot = '',
    [string]$TitleExe = '',
    [string]$CacheDir = '',
    [ValidateSet('series-s', 'series-x')]
    [string]$Console = 'series-s',
    [ValidateSet('1920x1080', '2560x1440')]
    [string]$Output = '1920x1080',
    [double]$Scale = 0.5,
    [switch]$Quality,
    [switch]$SmoothMotion,
    [switch]$DryRun
)

$ErrorActionPreference = 'Stop'

$LocalRepoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
if (-not $RepoRoot) {
    $localExe = Join-Path $LocalRepoRoot 'target\release\victory.exe'
    $fallbackRepo = 'D:\gta6-re\vicky-cgbj'
    $fallbackExe = Join-Path $fallbackRepo 'target\release\victory.exe'
    if (Test-Path $localExe) {
        $RepoRoot = $LocalRepoRoot
    } elseif (Test-Path $fallbackExe) {
        $RepoRoot = $fallbackRepo
        Write-Host "[run-nvidia-fast] Using existing build at $RepoRoot (build this repo with: cargo build --release -j 1 -p victory)" -ForegroundColor Yellow
    } else {
        $RepoRoot = $LocalRepoRoot
    }
} else {
    $RepoRoot = (Resolve-Path $RepoRoot).Path
}

$ExePath = Join-Path $RepoRoot 'target\release\victory.exe'
$HashesFile = Join-Path $LocalRepoRoot 'scripts\rt_off_hashes.txt'
if (-not (Test-Path $HashesFile)) {
    $HashesFile = Join-Path $RepoRoot 'scripts\rt_off_hashes.txt'
}

if (-not (Test-Path $ExePath)) {
    throw "Missing victory.exe at $ExePath. Build first with: cargo build --release -j 1 -p victory"
}
$AgilityDir = Join-Path (Split-Path $ExePath) 'd3d12sdk'
$AgilityCore = Join-Path $AgilityDir 'D3D12Core.dll'
if (-not (Test-Path $AgilityCore)) {
    $AgilityScript = Join-Path $LocalRepoRoot 'scripts\get-agility-sdk.ps1'
    if (-not (Test-Path $AgilityScript)) {
        $AgilityScript = Join-Path $RepoRoot 'scripts\get-agility-sdk.ps1'
    }
    if (Test-Path $AgilityScript) {
        Write-Host "[run-nvidia-fast] Missing d3d12sdk\D3D12Core.dll (would fail with 0x887E0003). Auto-fetching Agility SDK 1.619.6..." -ForegroundColor Yellow
        & $AgilityScript -OutDir $AgilityDir
    }
    if (-not (Test-Path $AgilityCore)) {
        throw "Missing $AgilityCore (D3D12_ERROR_INVALID_REDIST 0x887E0003). Run .\scripts\get-agility-sdk.ps1 first."
    }
}
if (-not (Test-Path $HashesFile)) {
    throw "Missing rt_off_hashes.txt at $HashesFile"
}

if (-not $TitleExe) {
    $candidates = @()
    if ($env:VICTORY_GAME_DIR) {
        $candidates += (Join-Path $env:VICTORY_GAME_DIR 'game_scarlett_master_nowd.exe')
        $candidates += (Join-Path $env:VICTORY_GAME_DIR 'game_scarlett_master.exe')
    }
    $localToml = Join-Path $RepoRoot 'victory.toml'
    if (Test-Path $localToml) {
        foreach ($line in Get-Content $localToml) {
            if ($line -match "^\s*game_dir\s*=\s*['""]([^'""]+)['""]") {
                $cfgDir = $Matches[1]
                $candidates += (Join-Path $cfgDir 'game_scarlett_master_nowd.exe')
                $candidates += (Join-Path $cfgDir 'game_scarlett_master.exe')
            }
        }
    }
    $candidates += 'E:\GTA6.Xbox.v1.0.2.1-CYBERLEEK\game_scarlett_master_nowd.exe'
    $candidates += 'E:\GTA6.Xbox.v1.0.2.1-CYBERLEEK\game_scarlett_master.exe'
    $TitleExe = ($candidates | Where-Object { Test-Path $_ } | Select-Object -First 1)
    if (-not $TitleExe) {
        $TitleExe = 'E:\GTA6.Xbox.v1.0.2.1-CYBERLEEK\game_scarlett_master_nowd.exe'
    }
}
if (-not (Test-Path $TitleExe)) {
    throw "Missing title executable at $TitleExe. Pass -TitleExe <path-to-game_scarlett_master.exe> or set VICTORY_GAME_DIR."
}

if (-not $CacheDir) {
    if ($env:VICTORY_CACHE_DIR) {
        $CacheDir = $env:VICTORY_CACHE_DIR
    } elseif (Test-Path 'D:\victory-data\cache-cgbj-fast') {
        $CacheDir = 'D:\victory-data\cache-cgbj-fast'
    } elseif (Test-Path 'D:\victory-data\cache-cgbj-mc') {
        $CacheDir = 'D:\victory-data\cache-cgbj-mc'
    } else {
        $CacheDir = Join-Path $env:LOCALAPPDATA 'VIctory\cache-nvidia'
    }
}

$LogDir = if (Test-Path 'D:\gta6-re') {
    'D:\gta6-re'
} else {
    Join-Path $env:LOCALAPPDATA 'VIctory\logs'
}
if (-not (Test-Path $LogDir)) {
    New-Item -ItemType Directory -Path $LogDir -Force | Out-Null
}
$LogBase = if ($DryRun) { 'run-nvidia-fast.dryrun' } else { 'run-nvidia-fast' }
$LogPath = Join-Path $LogDir "$LogBase.log"
$ErrPath = Join-Path $LogDir "$LogBase.err.log"

$hashes = @(
    Get-Content $HashesFile |
        ForEach-Object { $_ -split ',' } |
        ForEach-Object { $_.Trim() } |
        Where-Object { $_ -and -not $_.StartsWith('#') }
)
if ($hashes.Count -eq 0) {
    throw "No pipeline hashes parsed from $HashesFile"
}

$env:VICTORY_LOG = 'info'
$env:XCL_NETWORK = '0'
$env:VICTORY_D3D12_ADAPTER = 'NVIDIA'
$env:VICTORY_CACHE_DIR = $CacheDir

$env:VICTORY_OUTPUT = $Output
$env:VICTORY_FRAME_INTERVAL_MS = '16.667'
$env:VICTORY_PANEL = if ($env:VICTORY_PANEL) { $env:VICTORY_PANEL } else { '0' }
$env:VICTORY_MOUSE_LOOK = if ($env:VICTORY_MOUSE_LOOK) { $env:VICTORY_MOUSE_LOOK } else { '1' }
$env:VICTORY_MOUSE_SENS = if ($env:VICTORY_MOUSE_SENS) { $env:VICTORY_MOUSE_SENS } else { '1.0' }
$env:VICTORY_MOUSE_INVERT_Y = if ($env:VICTORY_MOUSE_INVERT_Y) { $env:VICTORY_MOUSE_INVERT_Y } else { '0' }

$env:VICTORY_UPSCALER = '6'
$env:VICTORY_RENDER_SCALE = ([string]$Scale).Replace(',', '.')
$env:VICTORY_SLOT_REMAP = '1'
$env:VICTORY_HEAP_BANKS = '3'
$env:VICTORY_BVH_MISS = '1'
$env:VICTORY_NAN_SCRUB = '1'

$env:VICTORY_AMBIENT = '0.8'
$env:VICTORY_AMBIENT_AIR = '1.5'
$env:VICTORY_AMBIENT_MOST = '2'

$env:VICTORY_SKIP_PIPELINES = ($hashes -join ',')
$env:VICTORY_PIPELINE_WAIT_MS = '25'
$env:VICTORY_PIPELINE_WORKERS = '4'
$env:VICTORY_PLACED_ORDER = '1'
$env:VICTORY_NATIVE_BARRIERS = '1'
Remove-Item Env:VICTORY_BARRIER_TEST -ErrorAction SilentlyContinue

$env:VICTORY_LEVELS_ON_DEMAND = '0'
$env:VICTORY_CARD_BUFFERS_MIN_KB = '64'
$env:VICTORY_CARD_BUFFERS_MIN_MB = '1'
$env:VICTORY_CARD_BUFFERS_MB = '1536'
$env:VICTORY_COMPOSE = '1'
$env:VICTORY_WAIT_RESIDENT = '0'
$env:VICTORY_PORT_CONSTANTS = '1'
$env:VICTORY_PORT_CBV = '0' # NVIDIA 1M view-heap cap forces raw_twins=false; PORT_CBV=1 nulls non-256B CBVs -> B&W
$env:VICTORY_PORT_GUARD = '1'
$env:VICTORY_PORT_GUARD_PLAN = '1'
$env:VICTORY_PORT_JOURNAL = '1'

$env:VICTORY_WIDE_BACK = '1'
$env:VICTORY_CARD_REHASH_RUNS = '1'
$env:VICTORY_JOIN_COPIES = '1'
$env:VICTORY_INDIRECT_WINDOW = '1'
$env:VICTORY_RELAY_COPY = '1'
$env:VICTORY_INDIRECT_BATCH = '2'
$env:VICTORY_MAP_THREADS = '1'
$env:VICTORY_TITLE_PRIORITIES = '0'
$env:VICTORY_CARD_AWAKE = '1'
$env:VICTORY_SHOW_AFTER = '1'
$env:VICTORY_THIN = '1'
$env:VICTORY_JOIN_PARTS = '8'
$env:VICTORY_UPLOAD_AHEAD = '1'
$env:VICTORY_RECYCLE_LANDED_EARLY = '1'
$env:VICTORY_READBACK_POOL = '1'

if ($Quality) {
    $env:VICTORY_RELAY = '1'
    $env:VICTORY_CARD_STEPS = '0'
    $env:VICTORY_TEXTURE_NO_WAIT = '0'
    $mode = 'Quality (RELAY=1, CARD_STEPS=0, TEXTURE_NO_WAIT=0)'
} else {
    $env:VICTORY_RELAY = '2'
    $env:VICTORY_CARD_STEPS = '2'
    $env:VICTORY_TEXTURE_NO_WAIT = '0'
    $mode = 'Fast (RELAY=2, CARD_STEPS=2, TEXTURE_NO_WAIT=0)'
}

$LaunchExe = $ExePath
if ($SmoothMotion) {
    $smPath = Join-Path (Split-Path $ExePath) 'victory_sm.exe'
    if (-not (Test-Path $smPath) -or (Get-Item $smPath).LastWriteTime -lt (Get-Item $ExePath).LastWriteTime -or (Get-Item $smPath).Length -ne (Get-Item $ExePath).Length) {
        Copy-Item $ExePath $smPath -Force
    }
    $LaunchExe = $smPath
}

Write-Host "[run-nvidia-fast] Mode: $mode | Console: $Console | Output: $($env:VICTORY_OUTPUT) | Scale: $($env:VICTORY_RENDER_SCALE) | NativeBarriers: $($env:VICTORY_NATIVE_BARRIERS) | Ports: 1,6,7 (PORT_CBV=0) | SmoothMotion: $($SmoothMotion.IsPresent) | SkipPipelines: $($hashes.Count) | Mouse: F2/MMB lock, Esc unlock, F7/F8 sens ($($env:VICTORY_MOUSE_SENS)) | Cache: $CacheDir | Log: $LogPath"

$argList = @('run')
if (-not $DryRun) { $argList += '--execute' }
$argList += @('--console', $Console, $TitleExe)

$proc = Start-Process -FilePath $LaunchExe `
    -ArgumentList $argList `
    -WorkingDirectory $RepoRoot `
    -RedirectStandardOutput $LogPath `
    -RedirectStandardError $ErrPath `
    -PassThru `
    -WindowStyle Normal
try { $proc.PriorityClass = 'AboveNormal' } catch {}

if ($DryRun) {
    $proc.WaitForExit()
    Write-Host "[run-nvidia-fast] DryRun exit code: $($proc.ExitCode)"
    exit $proc.ExitCode
}

Start-Sleep -Seconds 2
if ($proc.HasExited) {
    Write-Host "[run-nvidia-fast] victory.exe exited early with code $($proc.ExitCode)!" -ForegroundColor Red
    if (Test-Path $LogPath) { Get-Content $LogPath -Tail 30 }
    if (Test-Path $ErrPath) { Get-Content $ErrPath -Tail 30 }
    exit $proc.ExitCode
}

Write-Host "[run-nvidia-fast] Launched victory.exe PID=$($proc.Id) (AboveNormal). Logs: $LogPath / $ErrPath"
