//! `victory play`, and `victory` started with no command at all (a double
//! click): the title with the settings this fork plays with, so that the
//! program alone, put beside the title's `game_scarlett_master.exe`, is all
//! there is to start. What `scripts\run-nvidia-fast.ps1` does, built in.
//!
//! Nothing is written beside the program or into the title's folder: the
//! shader cache and a log for every start are kept under
//! `%LOCALAPPDATA%\VIctory`.

use std::collections::BTreeMap;
use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// The title's executable, looked for beside this program (the first that
/// is there).
const GAMES: [&str; 2] = ["game_scarlett_master_nowd.exe", "game_scarlett_master.exe"];

/// The pipelines left out (`VICTORY_SKIP_PIPELINES`): the title's ray
/// tracing ones.
const RT_OFF: &str = include_str!("../../../scripts/rt_off_hashes.txt");

/// The settings the title is started with (`scripts\run-nvidia-fast.ps1`
/// has the same).
const SETTINGS: &[(&str, &str)] = &[
    ("XCL_NETWORK", "0"),
    ("VICTORY_D3D12_ADAPTER", "NVIDIA"),
    ("VICTORY_FRAME_INTERVAL_MS", "16.667"),
    ("VICTORY_PANEL", "1"),
    // The title's own upscaler, at the scale asked for
    ("VICTORY_UPSCALER", "6"),
    // The view heap of an NVIDIA card: slots remapped, three banks
    ("VICTORY_SLOT_REMAP", "1"),
    ("VICTORY_HEAP_BANKS", "3"),
    ("VICTORY_BVH_MISS", "1"),
    ("VICTORY_NAN_SCRUB", "1"),
    // Light without the title's ray tracing
    ("VICTORY_AMBIENT", "0.8"),
    ("VICTORY_AMBIENT_AIR", "1.5"),
    ("VICTORY_AMBIENT_MOST", "2"),
    // Pipelines
    ("VICTORY_PIPELINE_WAIT_MS", "25"),
    ("VICTORY_PIPELINE_WORKERS", "4"),
    ("VICTORY_PLACED_ORDER", "1"),
    ("VICTORY_NATIVE_BARRIERS", "1"),
    // Buffers and textures
    ("VICTORY_LEVELS_ON_DEMAND", "0"),
    ("VICTORY_CARD_BUFFERS_MIN_KB", "64"),
    ("VICTORY_CARD_BUFFERS_MIN_MB", "1"),
    ("VICTORY_CARD_BUFFERS_MB", "1536"),
    ("VICTORY_COMPOSE", "1"),
    ("VICTORY_WAIT_RESIDENT", "0"),
    ("VICTORY_TEXTURE_NO_WAIT", "0"),
    // Renderer ports 1, 6 and 7. (PORT_CBV stays 0: with the view heap
    // capped at a million there are no raw twins, and 1 loses the colours.)
    ("VICTORY_PORT_CONSTANTS", "1"),
    ("VICTORY_PORT_CBV", "0"),
    ("VICTORY_PORT_GUARD", "1"),
    ("VICTORY_PORT_GUARD_PLAN", "1"),
    ("VICTORY_PORT_JOURNAL", "1"),
    // Submission and copy machinery
    ("VICTORY_WIDE_BACK", "1"),
    ("VICTORY_CARD_REHASH_RUNS", "1"),
    ("VICTORY_JOIN_COPIES", "1"),
    ("VICTORY_INDIRECT_WINDOW", "1"),
    ("VICTORY_RELAY_COPY", "1"),
    ("VICTORY_INDIRECT_BATCH", "2"),
    ("VICTORY_MAP_THREADS", "1"),
    ("VICTORY_TITLE_PRIORITIES", "0"),
    ("VICTORY_CARD_AWAKE", "1"),
    ("VICTORY_SHOW_AFTER", "1"),
    ("VICTORY_THIN", "1"),
    ("VICTORY_JOIN_PARTS", "8"),
    ("VICTORY_UPLOAD_AHEAD", "1"),
];

/// Settings a start keeps when its surroundings already have them.
const UNLESS_SET: &[(&str, &str)] =
    &[("VICTORY_MOUSE_LOOK", "1"), ("VICTORY_MOUSE_SENS", "1.0"), ("VICTORY_MOUSE_INVERT_Y", "0")];

const CONSOLES: [&str; 2] = ["series-s", "series-x"];
const OUTPUTS: [&str; 2] = ["1920x1080", "2560x1440"];

fn scale(spec: &str) -> Result<f32, String> {
    victory_loader::resolution::parse_render_scale(spec).ok_or_else(|| "expected 0.5 to 1.0".to_owned())
}

#[derive(clap::Args, Clone, Debug)]
pub(crate) struct Play {
    /// The title's executable. Without it: `game_scarlett_master.exe` beside
    /// this program, else in the current folder, else in the configured
    /// game folder (`VICTORY_GAME_DIR`, `victory.toml`).
    exe: Option<PathBuf>,
    /// The console the title sees. 2560x1440 needs series-x (series-s keeps
    /// back buffers sized for 1080p).
    #[arg(long, default_value = "series-s", value_parser = CONSOLES)]
    console: String,
    /// The size of the picture.
    #[arg(long, default_value = "1920x1080", value_parser = OUTPUTS)]
    output: String,
    /// The scene's size as a part of the picture's, 0.5 to 1.0.
    #[arg(long, default_value = "0.5", value_parser = scale)]
    scale: f32,
    /// A better picture, slower (`VICTORY_RELAY=1`, `VICTORY_CARD_STEPS=0`).
    #[arg(long)]
    quality: bool,
    /// Stop after preparing and binding the title, without starting it.
    #[arg(long)]
    dry_run: bool,
    /// Add or override a switch, as often as needed: `--set VICTORY_X=1`.
    #[arg(long = "set", value_name = "NAME=VALUE")]
    more: Vec<String>,
}

/// `%LOCALAPPDATA%\VIctory`.
fn home() -> Result<PathBuf> {
    let base = std::env::var_os("LOCALAPPDATA").context("no %LOCALAPPDATA% to keep the cache and logs in")?;
    Ok(PathBuf::from(base).join("VIctory"))
}

/// The pipeline hashes of `scripts\rt_off_hashes.txt`, joined by commas.
fn skipped(text: &str) -> String {
    let hashes: Vec<&str> =
        text.lines().flat_map(|l| l.split(',')).map(str::trim).filter(|h| !h.is_empty() && !h.starts_with('#')).collect();
    hashes.join(",")
}

/// The settings of a start with these options.
fn settings(o: &Play) -> BTreeMap<String, String> {
    let mut s: BTreeMap<String, String> = SETTINGS.iter().map(|(k, v)| ((*k).to_owned(), (*v).to_owned())).collect();
    let mut set = |k: &str, v: &str| drop(s.insert(k.to_owned(), v.to_owned()));
    set("VICTORY_OUTPUT", &o.output);
    set("VICTORY_RENDER_SCALE", &o.scale.to_string());
    set("VICTORY_SKIP_PIPELINES", &skipped(RT_OFF));
    set("VICTORY_RELAY", if o.quality { "1" } else { "2" });
    set("VICTORY_CARD_STEPS", if o.quality { "0" } else { "2" });
    for (k, v) in o.more.iter().filter_map(|m| m.split_once('=')) {
        s.insert(k.trim().to_owned(), v.trim().to_owned());
    }
    s
}

/// `year-month-day-hour-minute-second` (UTC) of `seconds` since 1970.
fn stamp(seconds: u64) -> String {
    let (days, rest) = (seconds / 86_400, seconds % 86_400);
    // (Howard Hinnant's civil-from-days.)
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let (day, month) = (doy - (153 * mp + 2) / 5 + 1, if mp < 10 { mp + 3 } else { mp - 9 });
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}{month:02}{day:02}-{:02}{:02}{:02}", rest / 3600, rest % 3600 / 60, rest % 60)
}

/// A new log file's name under `logs`, the ten newest of ours kept. The
/// process id is part of it, so that two starts in one second do not share
/// a name.
fn log_file(logs: &Path) -> Result<PathBuf> {
    std::fs::create_dir_all(logs).with_context(|| format!("creating {}", logs.display()))?;
    let mut old: Vec<PathBuf> = std::fs::read_dir(logs)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.file_name().is_some_and(|n| n.to_string_lossy().starts_with("play-")))
        .collect();
    old.sort();
    for gone in old.iter().rev().skip(9) {
        let _ = std::fs::remove_file(gone);
    }
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
    Ok(logs.join(format!("play-{}-{}.log", stamp(now), std::process::id())))
}

/// The title's executable in `dir`, if one is there.
fn game_in(dir: &Path) -> Option<PathBuf> {
    GAMES.iter().map(|g| dir.join(g)).find(|p| p.is_file())
}

/// What a start needs once its settings are in the environment.
pub(crate) struct Ready {
    pub(crate) exe: PathBuf,
    pub(crate) series_s: bool,
    pub(crate) execute: bool,
}

/// Finds the title, puts the settings into the environment and names the
/// log file (`VICTORY_LOG_FILE`). Called first thing, before any thread of
/// ours is started and before logging is set up.
pub(crate) fn prepare(o: &Play) -> Result<Ready> {
    let program = std::env::current_exe().context("this program's own path")?;
    let here = program.parent().map(Path::to_path_buf).unwrap_or_default();
    let exe = match &o.exe {
        Some(exe) => exe.clone(),
        None => game_in(&here)
            .or_else(|| game_in(Path::new(".")))
            .or_else(|| victory_core::Config::load().ok()?.game_dir.and_then(|dir| game_in(&dir)))
            .with_context(|| {
                format!(
                    "no {game} beside this program ({}): put the program into your own dump's folder, \
                     or name the title's executable (`victory play <path to {game}>`)",
                    here.display(),
                    game = GAMES[1]
                )
            })?,
    };
    anyhow::ensure!(exe.is_file(), "no {}", exe.display());
    let exe = std::fs::canonicalize(&exe).unwrap_or(exe);
    // (Windows' own long form `\\?\C:\...` back to the plain one.)
    let exe = PathBuf::from(exe.to_string_lossy().trim_start_matches(r"\\?\").to_owned());

    let home = home()?;
    let mut s = settings(o);
    for (k, v) in UNLESS_SET {
        if std::env::var_os(k).is_none() {
            s.entry((*k).to_owned()).or_insert_with(|| (*v).to_owned());
        }
    }
    if std::env::var_os("VICTORY_CACHE_DIR").is_none() {
        s.entry("VICTORY_CACHE_DIR".to_owned())
            .or_insert_with(|| home.join("cache-nvidia").to_string_lossy().into_owned());
    }
    let log = log_file(&home.join("logs"))?;
    let line: Vec<String> =
        s.iter().filter(|(k, _)| k.as_str() != "VICTORY_SKIP_PIPELINES").map(|(k, v)| format!("{k}={v}")).collect();
    std::fs::write(&log, format!("# console={} switches: {}\n", o.console, line.join(" ")))
        .with_context(|| format!("writing {}", log.display()))?;
    println!("Log: {}", log.display());
    s.insert("VICTORY_LOG_FILE".to_owned(), log.to_string_lossy().into_owned());

    // SAFETY: the first thing this process does: no other thread of ours runs yet.
    unsafe {
        std::env::remove_var("VICTORY_BARRIER_TEST");
        for (k, v) in &s {
            std::env::set_var(k, v);
        }
    }
    Ok(Ready { exe, series_s: o.console == "series-s", execute: !o.dry_run })
}

/// After a start that went wrong in a window of its own (a double click):
/// the text stays until a key is pressed.
pub(crate) fn hold_window() {
    if std::io::stdin().is_terminal() {
        print!("Press Enter to close this window.");
        let _ = std::io::stdout().flush();
        let _ = std::io::stdin().read_line(&mut String::new());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options() -> Play {
        Play {
            exe: None,
            console: "series-s".to_owned(),
            output: "1920x1080".to_owned(),
            scale: 0.5,
            quality: false,
            dry_run: false,
            more: Vec::new(),
        }
    }

    #[test]
    fn a_plain_start_is_the_launchers() {
        let s = settings(&options());
        for (k, v) in SETTINGS {
            assert_eq!(s.get(*k).map(String::as_str), Some(*v), "{k}");
        }
        let get = |k: &str| s.get(k).map(String::as_str);
        assert_eq!((get("VICTORY_OUTPUT"), get("VICTORY_RENDER_SCALE")), (Some("1920x1080"), Some("0.5")));
        assert_eq!((get("VICTORY_RELAY"), get("VICTORY_CARD_STEPS")), (Some("2"), Some("2")));
        assert_eq!((get("VICTORY_PORT_CBV"), get("VICTORY_TEXTURE_NO_WAIT")), (Some("0"), Some("0")));
        // the mouse's settings are left to the surroundings
        assert_eq!(get("VICTORY_MOUSE_SENS"), None);
    }

    #[test]
    fn the_left_out_pipelines_are_the_scripts_list() {
        let s = settings(&options());
        let skip: Vec<&str> = s["VICTORY_SKIP_PIPELINES"].split(',').collect();
        assert_eq!(skip.len(), 1034);
        assert!(skip.iter().all(|h| h.len() == 16 && h.bytes().all(|b| b.is_ascii_hexdigit())));
        assert_eq!(skipped("# a note\r\n0a, 0b,\r\n\r\n0c\r\n"), "0a,0b,0c");
    }

    #[test]
    fn options_and_set_are_applied() {
        let mut o = options();
        (o.output, o.scale, o.quality) = ("2560x1440".to_owned(), 0.67, true);
        o.more = vec!["VICTORY_AMBIENT=0".to_owned(), "VICTORY_NEW = 2".to_owned(), "no value".to_owned()];
        let s = settings(&o);
        let get = |k: &str| s.get(k).map(String::as_str);
        assert_eq!((get("VICTORY_OUTPUT"), get("VICTORY_RENDER_SCALE")), (Some("2560x1440"), Some("0.67")));
        assert_eq!((get("VICTORY_RELAY"), get("VICTORY_CARD_STEPS")), (Some("1"), Some("0")));
        assert_eq!((get("VICTORY_AMBIENT"), get("VICTORY_NEW")), (Some("0"), Some("2")));
    }

    #[test]
    fn scales_are_checked() {
        assert_eq!(scale("0.67"), Ok(0.67));
        assert!(scale("0.4").is_err() && scale("x").is_err());
    }

    #[test]
    fn stamps_are_dates() {
        assert_eq!(stamp(0), "19700101-000000");
        assert_eq!(stamp(1_795_753_200), "20261127-042000");
        assert_eq!(stamp(951_782_400), "20000229-000000");
    }
}
