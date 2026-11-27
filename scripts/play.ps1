# Starts the title with the settings this fork is played with (README.md, "How to run it").
#   scripts\play.ps1 -Exe <path of your own copy's executable>
# -Safe         without the newer, faster ways (the first thing to try when something is wrong)
# -RayTracing   the title's own ray-traced indirect light: lit interiors, about a third slower
# -Laptop       an NVIDIA PC with 16 GB of RAM and a 6-8 GB card: the memory settings and the RAM throttle
#               (measured by TaynanGT/Vice-Tay on an RTX 3050 6 GB laptop, Series S profile; not measured on this fork)
# -Build        target\diag (the default, the build all measurements are of) or target\release
# -More         other settings: -More @{VICTORY_PANEL='1'}
# Run it from a normal window, not as administrator: the driver then keeps its shader cache elsewhere.
param(
    [Parameter(Mandatory)][string]$Exe,
    [switch]$Safe,
    [switch]$RayTracing,
    [switch]$Laptop,
    [ValidateSet('diag', 'release')][string]$Build = 'diag',
    [hashtable]$More = @{}
)
$settings = @{
    VICTORY_TEXTURE_NO_WAIT = '1'; VICTORY_RELAY = '2'; VICTORY_WIDE_BACK = '1'; VICTORY_CARD_REHASH_RUNS = '1'
    VICTORY_JOIN_COPIES = '1'; VICTORY_INDIRECT_WINDOW = '1'; VICTORY_RELAY_COPY = '1'; VICTORY_CARD_STEPS = '2'
    VICTORY_INDIRECT_BATCH = '2'; VICTORY_MAP_THREADS = '1'; VICTORY_TITLE_PRIORITIES = '0'; VICTORY_CARD_AWAKE = '1'
    VICTORY_SHOW_AFTER = '1'; VICTORY_AMBIENT = '0.8'; VICTORY_D3D12_DRED = 'light'; VICTORY_SKIP_PIPELINES = 'none'
}
if ($Safe) {
    $settings.VICTORY_UNDER_WIDE = '0'; $settings.VICTORY_LAND_ECHO = '0'
} else {
    $settings.VICTORY_THIN = '1'; $settings.VICTORY_JOIN_PARTS = '8'; $settings.VICTORY_UPLOAD_AHEAD = '1'
}
if ($Laptop) {
    # What makes the world load fit in 16 GB; these stay under -Safe too. Game files read past
    # Windows' file cache, title buffers used in place, windows outside buffers cut on 64 KiB with
    # their tails and up to 3 GB of them in the card's memory, small card copies of title buffers.
    $settings.VICTORY_DSTORAGE_IO = 'direct'; $settings.VICTORY_INPLACE = '2'
    $settings.VICTORY_WINDOW_KB = '64'; $settings.VICTORY_WINDOW_TAILS = '1'
    $settings.VICTORY_WINDOWS_BAR_MB = '3072'; $settings.VICTORY_CARD_BUFFERS_MB = '256'
    $settings.VICTORY_LEVELS_ON_DEMAND = '1'
    # A ceiling on the game's working set: the RAM less 11.6 GB, at least 3 GB (4,500 MB on 16 GB).
    $ram = (Get-CimInstance Win32_ComputerSystem -ErrorAction SilentlyContinue).TotalPhysicalMemory
    if ($ram) {
        $settings.VICTORY_WORKING_SET_MB = [string][math]::Max(3072, [int64]($ram / 1MB) - 11624)
    }
    # A 1080p output drawn at half size by the title's own upscaler.
    $settings.VICTORY_OUTPUT = '1920x1080'; $settings.VICTORY_UPSCALER = '6'; $settings.VICTORY_RENDER_SCALE = '0.5'
    # The RAM throttle: under 1.5 GB free, new buffers in RAM and the title's submissions wait a
    # little. If the throttled load trips the title's 30 s watchdog, add
    # -More @{VICTORY_RAM_KEEP_ALIVE='1'}.
    $settings.VICTORY_RAM_FLOOR_MB = '1500'; $settings.VICTORY_RAM_WAIT_MS = '500'
    $settings.VICTORY_RAM_WAIT_SHARE_MS = '900'; $settings.VICTORY_RAM_FRAME_WAIT_MS = '1000'
    $settings.VICTORY_WINDOWS_BAR_PERCENT = '92'
}
if ($RayTracing) { $settings.VICTORY_BVH_BUILDER = '1' }
foreach ($k in $More.Keys) { $settings[$k] = $More[$k] }

$victory = Join-Path $PSScriptRoot "..\target\$Build\victory.exe"
if (-not (Test-Path $victory)) { Write-Host "No $victory. Build it first (README.md)."; return }
if (-not (Test-Path $Exe)) { Write-Host "No $Exe."; return }
# (Other settings left over in this window go; where your files are stays.)
$paths = 'VICTORY_CONFIG', 'VICTORY_GAME_DIR', 'VICTORY_CACHE_DIR', 'VICTORY_DATA_DIR'
Get-ChildItem Env:VICTORY_* | Where-Object { $_.Name -notin $paths } | ForEach-Object { Remove-Item "Env:$($_.Name)" }
foreach ($k in $settings.Keys) { Set-Item "Env:$k" $settings[$k] }
$Exe = (Resolve-Path $Exe).Path
Push-Location (Join-Path $PSScriptRoot '..')   # victory.toml is looked for here
try { & $victory run $Exe --execute } finally { Pop-Location }
