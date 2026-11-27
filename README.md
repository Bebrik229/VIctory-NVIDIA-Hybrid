# VIctory-NVIDIA-Hybrid

**NVIDIA RTX 12 GB Hybrid Edition** — standalone project built on [VIctory](CREDITS.md) where other forks failed on NVIDIA: full working shaders, NVIDIA `1,000,000` descriptor heap cap breakthrough (`0` cap errors across `44M+` slot writes), full-color rendering verified via GPU backbuffer capture, Native D3D12 Barriers, and multi-core Ports 1, 6, 7 for GTA VI (`1.0.2.1` Xbox Series build on Windows x64).

**Zero game assets policy:** This repository contains **no game files, executables, shaders, screenshots, or dumps**. You must supply your own Xbox title dump at runtime. See [docs/asset-policy.md](docs/asset-policy.md).

---

## Русский обзор / Russian Highlights

**VIctory-NVIDIA-Hybrid** — самостоятельный проект на базе **VIctory** (с сохранением GPL-3.0-or-later и полным указанием авторов в [CREDITS.md](CREDITS.md)), который **прорвался на видеокартах NVIDIA RTX там, где другие форки падали или теряли цвет**:

- **Прорыв через лимит дескрипторов NVIDIA 1M+ (`SLOT_REMAP` `t251` + 3 банка):** виртуально-физическая переадресация слотов с grace-рекламацией и clock-вытеснением даёт **`0` ошибок нехватки дескрипторов** на `44M+` живых записях.
- **Шейдеры полностью работают + фикс ч/б картинки на NVIDIA:** `28,562` предкомпилированных шейдеров (`PORT_CBV=0 + SLOT_REMAP=1`), релизный `VICTORY_SKIP_PIPELINES` (`1,034` RT-off хэшей), а также найден и устранён баг с нулевыми CBV (`descriptor/host.rs`: защита `twin.is_some()` + `VICTORY_PORT_CBV=0` при `raw_twins=false`). **Полная цветность подтверждена прямым захватом backbuffer GPU в 2K открытом мире.**
- **Нативные D3D12-барьеры (`VICTORY_NATIVE_BARRIERS=1`):** отслеживание `uav_dirty` / `budget_dirty` по каждому `HostList`, разбор `D3D12_BARRIER_GROUP` с пропуском read-only переходов и точечные `uav_barrier_for(&res)` / `D3D12_BUFFER_BARRIER` (в живом 2K-прогоне: `429,175` отправлено / `213,931` отброшено / `500,617` точечных / `43,770` group-skipped).
- **Плавный Mouse Look + анти-статтер + Ports 1, 6, 7:** аналоговое управление мышью и 24 ПК-клавиши (`gameinput.rs`), многопоточные порты рендерера `1, 6, 7`, фикс утечки памяти DirectStorage.
- **Проверенный FPS (NVIDIA RTX 5070 12 GB, Ryzen 7 7800X3D, 32 GB RAM):**
  - `1080p` `Series S` (`Scale=0.5`): **`11.6–16.0 fps`** (медиана **`~14.5 fps`**, стабильно **`15.3–16.0 fps` / `62.5–65.5 ms`**, с NVIDIA Smooth Motion 2x ощущается как **`~29–32 fps`**).
  - `2K` (`2560x1440`) `Series X` (`Scale=0.67`, полный цвет, `PORT_CBV=0`): медиана **`7.9 fps`** (**`7.6–8.5 fps`**, `118–132 ms/f`, `0` OOM, `0` descriptor cap).
  - `2K` нативный (`2560x1440`) `Series X` (`Scale=1.0`): **`4.7–5.3 fps`** (`190–211 ms/f`, `9.99 GB` VRAM, `0` крашей).

---

## Why This Fork Exists

NVIDIA Direct3D 12 drivers enforce a hard `1,000,000` shader-visible CBV/SRV/UAV descriptor heap limit, whereas AMD RDNA 3 drivers allow larger heaps. On open-world scenes in GTA VI (`1.0.2.1`), the title allocates past 1M descriptor slots and issues dense cross-pass barriers, causing other forks to hit `title physical descriptor cap reached`, `no free dynamic`, VRAM/RAM OOM on 12 GB / 32 GB systems, or severe GPU queue bubbles from global `UAV_BARRIER(NULL)` stalls.

**VIctory-NVIDIA-Hybrid** solves all four at the emulator/D3D12 backend level while integrating multi-core renderer ports 1, 5, 6, and 7.

---

## Core Engineering Breakthroughs

### 1. NVIDIA 1M+ Descriptor Heap Bypass (`SLOT_REMAP` `t251` + 3 Banks + Grace Reclaim & Eviction)

- **Files:** `crates/victory-gpu/src/descriptor/host.rs`, `crates/victory-shader/src/emit/shader.rs`, `crates/victory-gpu/src/shader_cache.rs`
- **How it works:**
  - `VICTORY_SLOT_REMAP=1` injects Root SRV `t251` (`SLOT_REMAP`) in `ROOT_CBV_SPACE` pointing to a `2^21` (`2,097,152`-entry, `8 MiB`) virtual-to-physical descriptor remap table (`SLOT_REMAP_COUNT`).
  - `VICTORY_HEAP_BANKS=3` splits shader-visible dynamic tables across 3 descriptor heap banks (`262,144` dynamic slots per bank, `REMAP_TITLE_VIRT_CAP = 1,310,720` virtual title slots) inside NVIDIA's `1,000,000` hardware heap limit.
  - `reclaim_title_slot` and clock-based `evict_stale_titles` recycle inactive physical slots after a `TITLE_RETIRE_GRACE = 6`-frame grace window using `Made::try_lock` (deadlock-free with `mirror`).
- **Observed result:** **`0` descriptor cap errors** across `44M+` live slot writes, with full compatibility across a `28,562`-file shader cache (`cache-cgbj-fast`, `PORT_CBV=0 + SLOT_REMAP=1`).

### 2. Full-Color NVIDIA Fix (`descriptor/host.rs` `twin.is_some()` + `VICTORY_PORT_CBV=0`)

- **Files:** `crates/victory-gpu/src/descriptor/host.rs`, `scripts/run-nvidia-fast.ps1`
- **Root cause:** On 1M-capped NVIDIA heaps (`RAW_TWINS = 1 << 21` exceeds `1,000,000`), `raw_views() == 0` disables raw descriptor twins, so translated DXIL shaders read CBVs from the normal descriptor slot `at`. With `VICTORY_PORT_CBV=1`, `make_twinned` wrote `D3D12_CONSTANT_BUFFER_VIEW_DESC::default()` (all-zero null CBV) into `at` for every non-256B-aligned CBV and skipped `if let Some(twin)`, causing shaders to read zeroes for material/lighting/tonemap buffers and output grayscale.
- **Fix:** Added `&& twin.is_some()` guard in `crates/victory-gpu/src/descriptor/host.rs` and defaulted `VICTORY_PORT_CBV=0` in `scripts/run-nvidia-fast.ps1`. **Full color visually verified via direct D3D12 swapchain backbuffer capture (`VICTORY_CAPTURE`) in 2K open world.**

### 3. Spec-Safe Native D3D12 Barriers (`VICTORY_NATIVE_BARRIERS=1` / `targeted` / `0`)

- **Files:** `crates/victory-gpu/src/host.rs`, `draw.rs`, `pipeline.rs`, `list.rs`, `list/thin.rs`, `indirect.rs`, `indirect/batch.rs`, `host/place.rs`, `host/card_steps.rs`, `ambient.rs`, `copystat.rs`
- **How it works:**
  - **Per-`HostList` dirty tracking:** `uav_dirty` and `budget_dirty` (`AtomicBool`) track actual UAV writes across both buffer stores (`Translated::stores`) and texture `image_store`/`image_atomic` instructions (`Translated::image_stores` over `Encoding::Mimg` in `pipeline.rs` / `draw.rs`).
  - **Enhanced Barrier parser (`D3D12_BARRIER_GROUP`):** `enhanced_barrier::parse` / `needs_uav_bool` in `list.rs` (packed into thin command streams via `pack_barrier_groups` in `list/thin.rs`) skips pure read-only transitions (`GroupSync::None`) while preserving synchronization for `ACCESS_COMMON`, `UNORDERED_ACCESS`, `RENDER_TARGET`, `DEPTH_STENCIL_WRITE`, `COPY_DEST`, `RESOLVE_DEST`, `STREAM_OUTPUT`, and `RAYTRACING_ACCELERATION_STRUCTURE_WRITE`.
  - **Targeted barriers:** `uav_barrier_for(&res)` and Enhanced `D3D12_BUFFER_BARRIER` (`buf_barrier` / `buffer_barriers` in `host/place.rs`) replace global `UAV_BARRIER(NULL)` and `wait_for_all` in indirect clamping, card steps, placed copies, and readback compare passes.
  - **Live 2K observed counters:** `emitted=429,175`, `elided=213,931`, `targeted=500,617`, `group_skipped=43,770`.
  - Modes: `1` (default, targeted barriers + title read-only elision), `targeted` (targeted host barriers, no title barrier elision), `0` (legacy global barriers).

### 4. Release `VICTORY_SKIP_PIPELINES`, Smooth Mouse Look & Anti-Stutter Tuning

- **Release RT-off Shader Skip (`VICTORY_SKIP_PIPELINES`):** `O(1)` `HashSet<String>` in `crates/victory-gpu/src/pipeline.rs` under `#[cfg(windows)]` works in release builds (no `diagnostics` build overhead), paired with `1,034` RT-off shader hashes in `scripts/rt_off_hashes.txt`.
- **Native Smooth Analog Mouse Look + 24-key PC Controls (`crates/victory-hle/src/modules/gameinput.rs`):**
  - Mouse-delta-to-right-stick smoothing (`VICTORY_MOUSE_LOOK=1`, `VICTORY_MOUSE_SENS=1.0`, `VICTORY_MOUSE_INVERT_Y=0`): **`F2` / Middle Mouse** locks/unlocks cursor, **`Esc` / Alt+Tab (focus loss)** unlocks, **`F7` / `F8`** adjusts sensitivity (`0.2`–`5.0`).
  - `RMB` = LT, `LMB` = RT, `X1` = B, `X2` = R3, plus 24-entry `FALLBACK_BUTTONS` keyboard mapping (`WASD` move, `Space`/`Left Shift` = A, `R`/`Backspace` = B, `X` = X, `G`/`Y` = Y, `Q`/`E` = LB/RB, `F`/`Left Ctrl` = LS, `H` = RS, `V`/`Tab` = View, `Esc` = Menu, `1`–`4` / Arrows = D-Pad).
- **Anti-Stutter Tuning:** `VICTORY_PIPELINE_WAIT_MS=25`, `VICTORY_PIPELINE_WORKERS=4`, `VICTORY_PLACED_ORDER=1`.
- **F3 stats panel in every build (`VICTORY_PANEL=1` shows it from the start):** frame rate and frame time, draws, how busy the GPU and the submitting thread are, copies, RAM and VRAM; the recording time is counted in `diagnostics` builds only.
- **Water light switch (`VICTORY_WATER_LIGHT=1`):** the water's two tiled light programs (`CS_TransmissionVolumetrics_Tiled` `5dc3b25986622949`, `_Tiled_LocalLights` `6f225b641c5b0708`) are left out on 32-lane (NVIDIA) hosts, where their wave-wide form leaves whole tiles of open water dark and costs about 15 ms a frame (measured in `trustno1-coder/VIctory-NVIDIA` on an RTX 5070); `VICTORY_WATER_LIGHT=1` runs them where they translate.

### 5. Integrated Multi-Core Renderer Ports 1, 6, 7 & DirectStorage OOM Fix

Integrated from `DoubleGhosty/cool_fox` (`Esteban Viera`, `Xyzstk`, `DoubleGhosty`, `Duff`, `fxceless`):
- **Port 1 (`VICTORY_PORT_CONSTANTS=1`):** raw constant buffer reads (Port 5 `VICTORY_PORT_CBV=0` is disabled on NVIDIA 1M-capped heaps and guarded by `twin.is_some()` in `descriptor/host.rs`).
- **Port 6 (`VICTORY_PORT_GUARD=1`, `VICTORY_PORT_GUARD_PLAN=1`):** shared guard lock across faults/refreshes and parallel submission refresh planning.
- **Port 7 (`VICTORY_PORT_JOURNAL=1`):** partitioned journal tracking and early descriptor sync.
- **DirectStorage memory fix (`6d8c299`):** compressed bytes freed once inflated; walls trace file list retained only when tracing is enabled.
- **Additional fidelity & stability fixes:** shared native descriptor snapshots across graphics stages (`eb4e048`), aligned water-height texture readback for boat missions (`09a9b8b`), low-address placeholders for in-place graphics buffers (`c397185`), sparse mip allocations (`a9e830c`), idle screen-probe / broken hair pass skipping + default ambient estimate (`67ffa6d`), phi-selected immediate samplers (`989190b`), and opt-in release GPU backbuffer capture (`VICTORY_CAPTURE`).

---

## Measured Benchmarks

Test platform: **AMD Ryzen 7 7800X3D**, **NVIDIA RTX 5070 12 GB**, **32 GB RAM**, GTA VI Xbox build (`cc5d9563...`), open-world prologue:

| Build / Configuration | Profile & Resolution | Frame Rate & Frame Time | Key Metrics |
|---|---|---|---|
| `avatarmanz/VIctory-7900XTX` baseline | 1080p | `7.0 fps` (`143.2 ms`) | `preparing 19.7 ms`, `copy-back 10.1 ms` |
| `vicky-cgbj` Ports 1, 6, 7 | 1080p, `Series S` | `10.5 fps` (`95.3 ms`) | `GPU 52.7 ms`, `Commit 16.90 GB` |
| `vicky-cgbj` Ports 1, 5, 6, 7 | 1080p, `Series S` | `11.4 fps` median (`87.5 ms`, peak `13.1 fps`) | `GPU 47.2 ms` |
| **`VIctory-NVIDIA-Hybrid` + Native Barriers (pre-PR #1/`#26`)** | **1080p `Scale=0.5`, `Series S`** | **`11.6–16.0 fps`** (median **`~14.5 fps`**, steady **`15.3–16.0 fps` / `62.5–65.5 ms`**) | `5.1k–5.7k draws/f`, `GPU wait 0.0–10.4 ms`, **`0` OOM, `0` descriptor cap** (`~29–32 fps` perceived with NVIDIA Smooth Motion 2x) |
| **`VIctory-NVIDIA-Hybrid` Latest (`#12`/`#21`/`#32`/`#35`/`#37`/`#44`/`#46`, Full Color)** | **1080p `Scale=0.5`, `Series S`** | **`9.3–16.1 fps`** (`62.0–107.1 ms/f`: `13.0–16.1 fps` open outdoor, `9.3–10.1 fps` `4.4k` draws dock/interior) | **CPU `preparing` cut to `18.0–20.5 ms`**, **`96–97%` GPU utilization**, `0` OOM, `0` descriptor cap |
| **`VIctory-NVIDIA-Hybrid` 2K Full Color** | **`Series X`, `2560x1440`, `Scale=0.67`, `PORT_CBV=0`** | **`7.9 fps` median** (`7.6–8.5 fps`, `118–132 ms/f`) | `8,220 draws/f`, `22.89 GB WS`, `20.87 GB Commit`, **`0` OOM, `0` descriptor cap, full color verified** |
| `VIctory-NVIDIA-Hybrid` Native 2K | `Series X`, `2560x1440`, `Scale=1.0` | `4.7–5.3 fps` (`190–211 ms/f`) | `12.4k–13.0k draws/f`, `9.99 GB` VRAM, `0` crashes |

> **Tip for 12 GB RTX cards:** Default `run-nvidia-fast.ps1` uses `-Console series-s -Output 1920x1080 -Scale 0.5` for maximum fluidity (`~14.5–16.0 fps`, or `~29–32 fps` with `-SmoothMotion`). For 2K (`2560x1440`), always use `-Console series-x -Output 2560x1440 -Scale 0.67` (`series-s` allocates a smaller back-buffer pool sized for 1080p).

---

## Workspace Structure

- `apps/victory` — CLI front end (`victory`)
- `crates/victory-core`, `victory-pe`, `victory-loader`, `victory-hle`, `victory-gpu`, `victory-shader`, `victory-formats`, `victory-pipeline`, `victory-game` — core runtime, HLE, D3D12 backend, and GFX10-to-DXIL shader translator
- `xtask` — repository policy verification (`cargo xtask policy`)
- `scripts/get-agility-sdk.ps1` — fetches DirectX 12 Agility SDK `1.619.6` (`D3D12Core.dll`) into `target\release\d3d12sdk\`
- `scripts/run-nvidia-fast.ps1` — portable NVIDIA Hybrid launcher (`PORT_CBV=0` full-color fix + auto-fetches Agility SDK if missing)
- `scripts/rt_off_hashes.txt` — `1,034` RT-off shader pipeline hashes

---

## Integrated NVIDIA & Performance Upgrades

- **NVIDIA bitwise `rndne_f32` & pipeline state + parallel slot reuse (`#1`, `#2`, `#3`):** IEEE-754 bitwise round-to-nearest-even for NVIDIA DXIL drivers, hardened descriptor slot reuse under parallel split submissions, and Port 1 indirect argument ordering (fixes palm/lamp flicker).
- **Release F3 stats panel & 32-lane water light skip (`#26`):** toggleable F3 overlay in every build + skips 64-lane-wide water tiled light computes on 32-lane NVIDIA hosts (`VICTORY_WATER_LIGHT=1` overrides).
- **AVX2/SSE2 micro-tile texture detiler (`#44`) + O(dirty) `shadow.rs` AVX2 (`#46`) + CPU/relay optimization wave (`#12`, `#13`, `#21`, `#28`, `#32`, `#35`, `#37`, `#40`, `#45`):** faster CPU texture detiling (`DetilePool`), O(dirty) shadow page planning + AVX2 `Shadow::take`, allocation-free write-watch shard walk (`#35`), uncovered-only title memory zeroing (`#28`), readback scratch/range merge reuse (`#32`) + pooled readback buffers (`VICTORY_READBACK_POOL=1`, `#12`), early relay command list recycling (`VICTORY_RECYCLE_LANDED_EARLY=1`, `#21`), batched CBV fill planning (`#37`), single-guard fast path, pre-enqueue `VICTORY_SKIP_PIPELINES` across `run` & `warm`, and resilient pipeline cache saves.

---

## Build & Launch

### Prerequisites

- Windows 10/11 x64, AVX2 CPU, Direct3D 12 GPU (Shader Model 6.6 + Mesh Shaders)
- Rust toolchain pinned in `rust-toolchain.toml` (`1.98.1`, `x86_64-pc-windows-msvc`)
- MSVC C++ Build Tools + Windows SDK
- Your own complete Xbox title dump (outside the repository)

### 1. Build Release Binary

```powershell
cargo build --release -j 1 -p victory
```

### 2. Install DirectX 12 Agility SDK 1.619.6 (Required — prevents `0x887E0003`)

`victory.exe` exports `D3D12SDKVersion = 619` and `D3D12SDKPath = ".\d3d12sdk\"` (`ID3D12Device13` / `OpenExistingHeapFromAddress1` for `VICTORY_PORT_CONSTANTS` / `VICTORY_INPLACE`). **Without `d3d12sdk\D3D12Core.dll` beside `victory.exe`, startup fails at DXR/root signature serialization with `D3D12_ERROR_INVALID_REDIST` (`0x887E0003`).**

`.\scripts\run-nvidia-fast.ps1` downloads it automatically on first run, or install it manually:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\get-agility-sdk.ps1
```

### 2. Launch (Recommended RTX 12 GB Fast Preset)

```powershell
# Default: -Console series-s -Output 1920x1080 -Scale 0.5
.\scripts\run-nvidia-fast.ps1 -TitleExe "D:\Games\GTA6\game_scarlett_master.exe"

# Enable NVIDIA Smooth Motion 2x copy (victory_sm.exe)
.\scripts\run-nvidia-fast.ps1 -TitleExe "D:\Games\GTA6\game_scarlett_master.exe" -SmoothMotion

# Higher quality (RELAY=1, CARD_STEPS=0) or 2K scaled (requires -Console series-x)
.\scripts\run-nvidia-fast.ps1 -Quality -Console series-x -Output 2560x1440 -Scale 0.67

# Dry run (translation/init check without executing title entry)
.\scripts\run-nvidia-fast.ps1 -DryRun
```

> All launcher parameters (`-RepoRoot`, `-TitleExe`, `-CacheDir`, `-Console`, `-Output`, `-Scale`, `-Quality`, `-SmoothMotion`, `-DryRun`) are optional; `VICTORY_GAME_DIR` and `VICTORY_CACHE_DIR` environment variables or `victory.toml` are also supported.

### 3. Pre-warm Shaders (Optional)

```powershell
target\release\victory.exe shaders <path-to-shader-archives>
```

---

## Repository

- Public standalone repository: [https://github.com/Bebrik229/VIctory-NVIDIA-Hybrid](https://github.com/Bebrik229/VIctory-NVIDIA-Hybrid)

---

## Credits & License

See [CREDITS.md](CREDITS.md) for the complete upstream project and contributor attribution.

Licensed under **GPL-3.0-or-later** ([LICENSE](LICENSE)), matching upstream VIctory. The single non-GPL file is `crates/victory-formats/src/bcpack-tables.bin` (constant BCPack format tables, documented in [docs/asset-policy.md](docs/asset-policy.md)).
