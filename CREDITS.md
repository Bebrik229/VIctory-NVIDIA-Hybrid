# Credits & Attribution

**VIctory-NVIDIA-Hybrid** builds on the collective reverse-engineering, HLE, D3D12, and shader-recompilation work of the VIctory community. Every upstream project, fork, and contributor from the repository history is acknowledged below.

## Upstream Repositories & Key Engineering Contributions

- **[Marvisak/VIctory](https://github.com/Marvisak/VIctory)** (`Marvis`) — Original VIctory Xbox Series compatibility layer: PE loader, HLE services, D3D12 backend, and GFX10-to-DXIL shader recompiler.
- **[Pavl3000/cgbj-fork](https://github.com/Pavl3000/cgbj-fork)** (`Pavel`) — Base hybrid tree preserving `codec-d`'s residency defaults (`VICTORY_RESIDENCY`), `VICTORY_POOL_SMALL=0`, and wave-based upstream cherry-picks.
- **[DoubleGhosty/cool_fox](https://github.com/DoubleGhosty/cool_fox)** (`DoubleGhosty`, `Xyzstk`, `Duff`, `fxceless`, `Esteban Viera`) —
  - Multi-core Renderer **Ports 1, 5, 6, and 7** (`VICTORY_PORT_CONSTANTS`, `VICTORY_PORT_CBV`, `VICTORY_PORT_GUARD`, `VICTORY_PORT_GUARD_PLAN`, `VICTORY_PORT_JOURNAL`)
  - DirectStorage OOM fix (freeing compressed buffers once inflated; gating walls trace file list)
  - Shared native descriptor snapshots across graphics stages (`eb4e048`)
  - Aligned water-height texture readback for boat missions (`09a9b8b`)
  - Low-address placeholders for in-place graphics buffers (`c397185`), persistent head init (`37935ab`), and coalesced checksum comparisons (`5141f9a`)
  - Sparse mip allocations (`a9e830c`), idle screen-probe / broken hair pass skipping + default ambient estimate (`67ffa6d`), and phi-selected immediate samplers (`989190b`)
- **`codec-d/cool_fox`** (`codec_d`) — RTX 5090 fidelity & 32 GB RAM residency optimizations, Series S two-plane `RetailPlane` 72-byte stride compose fix, `TablePlan`, 256 MiB DirectStorage tuning, and `WaitOnAddress` fence wakes.
- **[avatarmanz/VIctory-7900XTX](https://github.com/avatarmanz/VIctory-7900XTX)** (`avatarmanz`) — Thin command lists (`VICTORY_THIN`), multi-threaded submission preparation (`VICTORY_JOIN_PARTS`), `VICTORY_CARD_STEPS`, relay/landing architecture, and frame replay diagnostics.
- **`Davidjc13/VIctory-7900XTX`** (`Davidjc13`) — Texture sweep / card-space wait optimizations and LDS atomic wave32 handling.
- **`trustno1-coder/VIctory-NVIDIA`** — NVIDIA descriptor/performance speedups.
- **[LooperSalty/AnyXboxX](https://github.com/LooperSalty/AnyXboxX)** — PS5 pad (DualSense, DualSense Edge) HID input (`gameinput/dualsense.rs`), via `trustno1-coder/VIctory-NVIDIA`.
- **[Belobug/VIctory-Belobug-AMD](https://github.com/Belobug/VIctory-Belobug-AMD)** (`beloooo`) — Indirect-light ambient stand-in discovery (`VICTORY_AMBIENT`), pipeline worker threading, and `romainhedouin`'s contributions.
- **`triippywhale`** — `fix/amd-view-heap-clamp` view heap clamping work.
- **[fxceless/victory-fork](https://github.com/fxceless/victory-fork)**, **[DexNom/VIctory](https://github.com/DexNom/VIctory)**, **[myokex/victory-amd](https://github.com/myokex/victory-amd)**, **[FQY55kai/ZJOU-VICTORY](https://github.com/FQY55kai/ZJOU-VICTORY)**, `Geckomacabre`, `cxy0072019-sketch`, `Spexivus`, `joroyt`, `tightfall`, `Joshua Odom` — Additional D3D12, texture readback, sparse texture, cube gradient, and guard improvements noted in upstream documentation.

## Contributors (`git shortlog`)

All contributors recorded in the repository's commit history:

- **Core & Fork Maintainers:** `Marvis`, `DoubleGhosty`, `Duff`, `fxceless`, `Xyzstk`, `codec_d`, `Pavel`, `Davidjc13`, `avatarmanz`, `Esteban Viera`
- **Community Contributors:** `Maraco91`, `Vyacheslav Venglovsky`, `Manuel Pérez`, `TG0000`, `marquezini`, `给伊莉雅补魔`, `quaternion`, `r4.`, `9hvn9yt2vk-netizen`, `Enzo Souza Duarte`, `FIocker`, `fuzeofficial` (`Fuzeofficial`), `tedin7`, `EatCodeSleep2`, `Luiz Neves` (`Luiz Gustavo Benicio Neves`)
- **Additional History Contributors:** `gomeaw`, `feanorsun`, `simon0302010`, `ifBars`, `星记`, `LF2000 Builder`, `Noname0202`, `fever308`, `AnoHunted`, `BastienRdz`, `Tex`, `Yoshi-233`, `jaschinski97-ops`, `raffclar`, `Adika713`, `AlanQiang0418`, `Andrea Casale`, `BaggyALPHA`, `ESAT`, `Exorion`, `Henrique Bueno`, `Jake`, `Jeremy Goldberg`, `KoLolX3`, `Louis Faust`, `Pedro Fortunato`, `Saitken91`, `UwUCammie`, `Vanthys`, `Yaekai`, `hophaver`, `lameloball2813-ui`, `nathanbzrr`, `sharan104`, `spacede`, `superboynick`, `zocram4cc`, `Abdellah K.`, `Advait Palve`, `AgneyaK`, `Aws`, `Bloxive`, `Calvix2k`, `Carlos Sant'Anna`, `Dathinel`, `David Palacios`, `Devin`, `Diego`, `Diogo Sousa`, `DocBlack89`, `DoubleD17`, `Eduardo Bertozzi`, `FictionHub`, `FrancescoGelosoEterea`, `Giuseppe Rainone`, `HailLucifer666`, `LUIS LARIOS`, `LaunchDoesntCode`, `Luiz Otavio Mello de Campos`, `Lux-Designs`, `Lyrnc`, `Maledetto`, `Mumogg`, `Nixitov`, `Nostyxx`, `NuraadGit`, `Omeya Sawant`, `SlopComp`, `Thomas Askharoun`, `Vax`, `ZazaJr24`, `alessiodeia`, `artor410`, `ashrafsdrop`, `broccolize`, `bvtterbroot`, `cxccxc`, `divyeshganta`, `fluffyflattershy-afk`, `heromike405-arch`, `imjustprism`, `ismailchaka1`, `itsRevela`, `julio`, `justrealyme`, `lordon1a`, `mattkajorinc`, `meower`, `mgunes1311-lab`, `moe-miau`, `nino-exe`, `oddiums`, `pes7`, `pgm1207`, `ryotdev`, `sawetco`, `slatefield`, `warmooff-ops`

## License Preservation

This project preserves the upstream **GNU General Public License v3.0 or later (`GPL-3.0-or-later`)** as found in [LICENSE](LICENSE).
Per [docs/asset-policy.md](docs/asset-policy.md), `crates/victory-formats/src/bcpack-tables.bin` contains non-copyrightable format constant tables and is excluded from the GPL license grant.
