@echo off
setlocal EnableExtensions
chcp 65001 >nul
set "SCRIPT_DIR=%~dp0"

rem ============================================================================
rem  VIctory-NVIDIA-Hybrid Quick Launcher (edit settings below as needed)
rem  Tip: leave GAME_EXE empty for auto-detect, or drag-and-drop
rem       game_scarlett_master.exe onto this .bat file.
rem ============================================================================
set "GAME_EXE="
set "CONSOLE=series-s"
set "OUTPUT=1920x1080"
set "SCALE=0.5"
set "SMOOTH_MOTION=0"
set "QUALITY=0"
set "HAIR=0"

rem --- Frame rate / NVIDIA barriers / CPU & RT optimizations ------------------
set "VICTORY_FRAME_INTERVAL_MS=0"
set "VICTORY_NATIVE_BARRIERS=targeted"
set "VICTORY_PORT_CONSTANTS=0"
set "VICTORY_PORT_RT=1"
set "VICTORY_REAPER=1"
set "VICTORY_COPY_AHEAD=0"
set "VICTORY_PRELUDE_POOL=1"
set "VICTORY_SECTION_MB=2"
set "VICTORY_THIN_THREADS=8"
set "VICTORY_RT_SKIP_DEAD=1"
set "VICTORY_BVH_NO_HIT=1"
set "VICTORY_PROBE_RAYS=1"
set "VICTORY_SAMPLERS_MADE=1"
set "VICTORY_MOTION_BLUR_OFF=1"
set "VICTORY_TABLE_MEMO=1"
set "VICTORY_TABLE_MEMO_SLOTS=1"
set "VICTORY_TABLE_NAMED_ONLY=1"
set "VICTORY_LIST_HOLD=1"
set "VICTORY_QUEUE_MARKS=1"
set "VICTORY_LIST_ALIAS_EXACT=1"
set "VICTORY_BIND_FILTER=1"
set "VICTORY_BIND_SAME=0"
set "VICTORY_HEAP_SYNC=1"
set "VICTORY_TEXTURE_STAGE_MS=1"
set "VICTORY_TEXTURE_CATCHUP=0"
set "VICTORY_HEAP_RECHECK=0"
set "VICTORY_CLEAN_STAMP=0"
set "VICTORY_SKIP_UNUSED_STAMPS=1"
set "VICTORY_ASK_SPREAD=0"
set "VICTORY_GUARD_REST=1"
set "VICTORY_E5_ALPHA_ONE=1"
set "VICTORY_BAKE_DELTA=1"
set "VICTORY_PREPARE_WIDE=1"
set "VICTORY_UNDER_FUSED=1"
set "VICTORY_REFRESH_ONCE=1"
set "VICTORY_NARROW_WRITES=1"
set "VICTORY_FILL_FROM_PLACED=1"
set "VICTORY_ASK_KEPT_EARLY=1"
set "VICTORY_ASK_ONCE_GUARDS=1"
set "VICTORY_CARD_LISTS=1"
set "VICTORY_WIDE_MERGE=1"
set "VICTORY_FILL_TITLE=0"
set "VICTORY_PIPELINE_BOOK=1"
set "VICTORY_HEAP_ROUND=0"
set "VICTORY_LATE_WAIT_LISTS=1024"
set "VICTORY_CARD_AWAKE=0"
set "VICTORY_JOIN_PARTS=16"
set "VICTORY_JOIN_LISTS=64"
set "VICTORY_BAKE_TEXTURES_MS=10000"

rem --- Lighting / controls / HUD ----------------------------------------------
set "VICTORY_AMBIENT=0.6"
set "VICTORY_AMBIENT_AIR=1.0"
set "VICTORY_AMBIENT_MARCH=1"
set "VICTORY_PC_CONTROLS=1"
set "VICTORY_MOUSE_LOOK=1"
set "VICTORY_MOUSE_SENS=1.0"
set "VICTORY_MOUSE_INVERT_Y=0"
set "VICTORY_PANEL=0"

rem --- Handle drag-and-drop .exe as %1 and forward any extra flags ------------
set "EXTRA_ARGS="
if /I "%~x1"==".exe" (
    if exist "%~1" (
        set "GAME_EXE=%~1"
        shift
    )
)
:collect_args
if "%~1"=="" goto args_done
set "EXTRA_ARGS=%EXTRA_ARGS% %1"
shift
goto collect_args
:args_done

set "PS_SWITCHES="
if /I "%SMOOTH_MOTION%"=="1" set "PS_SWITCHES=%PS_SWITCHES% -SmoothMotion"
if /I "%QUALITY%"=="1" set "PS_SWITCHES=%PS_SWITCHES% -Quality"
if /I "%HAIR%"=="1" set "PS_SWITCHES=%PS_SWITCHES% -Hair"

if defined GAME_EXE (
    powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%SCRIPT_DIR%scripts\run-nvidia-fast.ps1" -TitleExe "%GAME_EXE%" -Console "%CONSOLE%" -Output "%OUTPUT%" -Scale %SCALE% %PS_SWITCHES% %EXTRA_ARGS%
) else (
    powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%SCRIPT_DIR%scripts\run-nvidia-fast.ps1" -Console "%CONSOLE%" -Output "%OUTPUT%" -Scale %SCALE% %PS_SWITCHES% %EXTRA_ARGS%
)
set "RC=%ERRORLEVEL%"

if not "%RC%"=="0" (
    echo.
    echo [run-nvidia-fast.bat] Launch failed with exit code %RC%.
    if not "%NO_PAUSE%"=="1" pause
)
exit /b %RC%
