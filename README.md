# VIctory-NVIDIA-Hybrid

**NVIDIA RTX 12 GB Hybrid Edition** of VIctory — a Windows x64 Direct3D 12 Xbox Series compatibility layer tuned specifically to eliminate NVIDIA's `1,000,000` shader-visible descriptor heap cap, global D3D12 barrier pipeline stalls, and 12 GB VRAM / 32 GB RAM bottlenecks on GTA VI (Xbox `1.0.2.1` build).

**Zero game assets policy:** This repository contains **no game files, executables, shaders, or dumps**. You must supply your own Xbox title dump at runtime. See [docs/asset-policy.md](docs/asset-policy.md).

---

## Русский обзор / Russian Highlights

**VIctory-NVIDIA-Hybrid** — специализированный форк для видеокарт **NVIDIA RTX 12 GB** (и систем с 32 ГБ ОЗУ), устраняющий аппаратный потолок NVIDIA в 1 000 000 дескрипторов и глобальные барьеры D3D12:

- **Обход лимита дескрипторов 1M+ (`SLOT_REMAP` `t251` + 3 банка):** виртуально-физическая переадресация слотов с grace-рекламацией и clock-вытеснением даёт **`0` ошибок нехватки слотов** на >44M живых записях и полную совместимость с кэшем из `25,753` шейдеров.
- **Нативные D3D12-барьеры (`VICTORY_NATIVE_BARRIERS=1`):** отслеживание `uav_dirty` / `budget_dirty` по каждому `HostList`, разбор Enhanced Barrier (`D3D12_BARRIER_GROUP`) с пропуском read-only переходов и точечные `uav_barrier_for(&res)` / `D3D12_BUFFER_BARRIER` вместо глобальных `UAV_BARRIER(NULL)` и `wait_for_all`.
- **Плавный аналоговый Mouse Look + анти-статтер:** `VICTORY_SKIP_PIPELINES` в release (`1,034` RT-off хэшей), плавное управление мышью и 24 ПК-клавиши в `gameinput.rs`, а также многопоточные порты рендерера **1, 5, 6, 7** и фикс утечки памяти DirectStorage.
- **Производительность (RTX 5070 12 GB, 7800X3D, 32 GB RAM):** **`11.6–16.0 fps`** (медиана **`~14.5 fps`**, пики **`15.3–16.0 fps` / `62.5–65.5 ms`**) в 1080p `Series S` (`Scale=0.5`), с **NVIDIA Smooth Motion 2x** ощущается как **`~29–32 fps`**. Для 2K (`2560x1440`) используйте `-Scale 0.67` или профиль `series-s` 1080p.

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
- **Observed result:** **`0` descriptor cap errors** across `44M+` live slot writes, with full compatibility across a `25,753`-file shader cache.

### 2. Spec-Safe Native D3D12 Barriers (`VICTORY_NATIVE_BARRIERS=1` / `targeted` / `0`)

- **Files:** `crates/victory-gpu/src/host.rs`, `draw.rs`, `pipeline.rs`, `list.rs`, `list/thin.rs`, `indirect.rs`, `indirect/batch.rs`, `host/place.rs`, `host/card_steps.rs`, `ambient.rs`, `copystat.rs`
- **How it works:**
  - **Per-`HostList` dirty tracking:** `uav_dirty` and `budget_dirty` (`AtomicBool`) track actual UAV writes across both buffer stores (`Translated::stores`) and texture `image_store`/`image_atomic` instructions (`Translated::image_stores` over `Encoding::Mimg` in `pipeline.rs` / `draw.rs`).
  - **Enhanced Barrier parser (`D3D12_BARRIER_GROUP`):** `enhanced_barrier::parse` / `needs_uav_bool` in `list.rs` (packed into thin command streams via `pack_barrier_groups` in `list/thin.rs`) skips pure read-only transitions (`GroupSync::None`) while preserving synchronization for `ACCESS_COMMON`, `UNORDERED_ACCESS`, `RENDER_TARGET`, `DEPTH_STENCIL_WRITE`, `COPY_DEST`, `RESOLVE_DEST`, `STREAM_OUTPUT`, and `RAYTRACING_ACCELERATION_STRUCTURE_WRITE`.
  - **Targeted barriers:** `uav_barrier_for(&res)` and Enhanced `D3D12_BUFFER_BARRIER` (`buf_barrier` / `buffer_barriers` in `host/place.rs`) replace global `UAV_BARRIER(NULL)` and `wait_for_all` in indirect clamping, card steps, placed copies, and readback compare passes.
  - Modes: `1` (default, targeted barriers + title read-only elision), `targeted` (targeted host barriers, no title barrier elision), `0` (legacy global barriers).

### 3. Release `VICTORY_SKIP_PIPELINES`, Smooth Mouse Look & Anti-Stutter Tuning

- **Release RT-off Shader Skip (`VICTORY_SKIP_PIPELINES`):** `O(1)` `HashSet<String>` in `crates/victory-gpu/src/pipeline.rs` under `#[cfg(windows)]` works in release builds (no `diagnostics` build overhead), paired with `1,034` RT-off shader hashes in `scripts/rt_off_hashes.txt`.
- **Native Smooth Analog Mouse Look + 24-key PC Controls (`crates/victory-hle/src/modules/gameinput.rs`):**
  - Mouse-delta-to-right-stick smoothing (`VICTORY_MOUSE_LOOK=1`, `VICTORY_MOUSE_SENS=1.0`, `VICTORY_MOUSE_INVERT_Y=0`): **`F2` / Middle Mouse** locks/unlocks cursor, **`Esc` / Alt+Tab (focus loss)** unlocks, **`F7` / `F8`** adjusts sensitivity (`0.2`–`5.0`).
  - `RMB` = LT, `LMB` = RT, `X1` = B, `X2` = R3, plus 24-entry `FALLBACK_BUTTONS` keyboard mapping (`WASD` move, `Space`/`Left Shift` = A, `R`/`Backspace` = B, `X` = X, `G`/`Y` = Y, `Q`/`E` = LB/RB, `F`/`Left Ctrl` = LS, `H` = RS, `V`/`Tab` = View, `Esc` = Menu, `1`–`4` / Arrows = D-Pad).
- **Anti-Stutter Tuning:** `VICTORY_PIPELINE_WAIT_MS=25`, `VICTORY_PIPELINE_WORKERS=4`, `VICTORY_PLACED_ORDER=1`.

### 4. Integrated Multi-Core Renderer Ports 1, 5, 6, 7 & DirectStorage OOM Fix

Integrated from `DoubleGhosty/cool_fox` (`Esteban Viera`, `Xyzstk`, `DoubleGhosty`, `Duff`, `fxceless`):
- **Port 1 (`VICTORY_PORT_CONSTANTS=1`)** & **Port 5 (`VICTORY_PORT_CBV=1`):** raw constant buffer reads and in-place constant buffers.
- **Port 6 (`VICTORY_PORT_GUARD=1`, `VICTORY_PORT_GUARD_PLAN=1`):** shared guard lock across faults/refreshes and parallel submission refresh planning.
- **Port 7 (`VICTORY_PORT_JOURNAL=1`):** partitioned journal tracking and early descriptor sync.
- **DirectStorage memory fix (`6d8c299`):** compressed bytes freed once inflated; walls trace file list retained only when tracing is enabled.
- **Additional fidelity & stability fixes:** shared native descriptor snapshots across graphics stages (`eb4e048`), aligned water-height texture readback for boat missions (`09a9b8b`), low-address placeholders for in-place graphics buffers (`c397185`), sparse mip allocations (`a9e830c`), idle screen-probe / broken hair pass skipping + default ambient estimate (`67ffa6d`), and phi-selected immediate samplers (`989190b`).

---

## Measured Benchmarks

Test platform: **AMD Ryzen 7 7800X3D**, **NVIDIA RTX 5070 12 GB**, **32 GB RAM**, GTA VI Xbox build (`cc5d9563...`), open-world prologue:

| Build / Configuration | Profile & Resolution | Frame Rate & Frame Time | Key Metrics |
|---|---|---|---|
| `avatarmanz/VIctory-7900XTX` baseline | 1080p | `7.0 fps` (`143.2 ms`) | `preparing 19.7 ms`, `copy-back 10.1 ms` |
| `vicky-cgbj` Ports 1, 6, 7 | 1080p, `Series S` | `10.5 fps` (`95.3 ms`) | `GPU 52.7 ms`, `Commit 16.90 GB` |
| `vicky-cgbj` Ports 1, 5, 6, 7 | 1080p, `Series S` | `11.4 fps` median (`87.5 ms`, peak `13.1 fps`) | `GPU 47.2 ms` |
| **`VIctory-NVIDIA-Hybrid` + Native Barriers** | **1080p `Scale=0.5`, `Series S`** | **`11.6–16.0 fps`** (median **`~14.5 fps`**, **`15.3–16.0 fps` / `62.5–65.5 ms`**) | `5.1k–5.7k draws/f`, `GPU wait 0.0–10.4 ms`, **`0` OOM, `0` descriptor cap** (`~29–32 fps` perceived with NVIDIA Smooth Motion 2x) |
| `VIctory-NVIDIA-Hybrid` Native 2K | `Series X`, `2560x1440`, `Scale=1.0` | `4.7–5.3 fps` (`190–211 ms/f`) | `12.4k–13.0k draws/f`, `9.99 GB` VRAM, `0` crashes |

> **Tip for 12 GB RTX cards:** Default `run-nvidia-fast.ps1` uses `-Console series-s -Output 1920x1080 -Scale 0.5` for maximum fluidity (`~14.5–16.0 fps`, or `~29–32 fps` with `-SmoothMotion`). If you want `2560x1440` (`series-x`), pass `-Scale 0.67` to avoid 2K native fill-rate saturation.

---

## Workspace Structure

- `apps/victory` — CLI front end (`victory`)
- `crates/victory-core`, `victory-pe`, `victory-loader`, `victory-hle`, `victory-gpu`, `victory-shader`, `victory-formats`, `victory-pipeline`, `victory-game` — core runtime, HLE, D3D12 backend, and GFX10-to-DXIL shader translator
- `xtask` — repository policy verification (`cargo xtask policy`)
- `scripts/run-nvidia-fast.ps1` — portable NVIDIA Hybrid launcher
- `scripts/rt_off_hashes.txt` — `1,034` RT-off shader pipeline hashes

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

### 2. Launch (Recommended RTX 12 GB Fast Preset)

```powershell
# Default: -Console series-s -Output 1920x1080 -Scale 0.5
.\scripts\run-nvidia-fast.ps1 -TitleExe "D:\Games\GTA6\game_scarlett_master.exe"

# Enable NVIDIA Smooth Motion 2x copy (victory_sm.exe)
.\scripts\run-nvidia-fast.ps1 -TitleExe "D:\Games\GTA6\game_scarlett_master.exe" -SmoothMotion

# Higher quality (RELAY=1, CARD_STEPS=0) or 2K scaled
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

## Publishing to GitHub

```powershell
git remote add origin <URL>
git push -u origin main
```

---

## Credits & License

See [CREDITS.md](CREDITS.md) for the complete upstream project and contributor attribution.

Licensed under **GPL-3.0-or-later** ([LICENSE](LICENSE)), matching upstream VIctory. The single non-GPL file is `crates/victory-formats/src/bcpack-tables.bin` (constant BCPack format tables, documented in [docs/asset-policy.md](docs/asset-policy.md)).
