//! (Coder W) `VICTORY_CLEAN_STAMP` telemetry and mode switches (`0` off, `1` on, `2` verify).

use std::sync::Mutex;

/// `VICTORY_CLEAN_STAMP`: `0` off, `1` on (default when unset), `2` (`verify`) verify.
///
/// With it a copy keeps the last range [`crate::shadow::Shadow::nothing_new`] found clean and
/// the block-change counter (`crate::watch::block_seq_now`) read before it looked; asked again
/// for a range inside it with every block's counter still at most that number, the answer is
/// clean at once without taking `pending`, `copied`, or journal locks.
///
/// Sound: the answer reads only the journal, the published and beside bits (each change of
/// which bumps the block's counter), shaders' ranges in flight (`Shadow::writing` clears the
/// stamp), and the copy's own notes (which only grow more current). `verify` (`"verify"`):
/// runs the full look anyway and counts stamped answers the look did not confirm (must be 0).
pub(crate) fn clean_stamp() -> u8 {
    static MODE: std::sync::OnceLock<u8> = std::sync::OnceLock::new();
    *MODE.get_or_init(|| {
        let m = match std::env::var("VICTORY_CLEAN_STAMP").as_deref().map(str::trim) {
            Ok("0") => 0,
            Ok("verify") => 2,
            Ok("1") | Err(_) => 1,
            Ok(_) => 0,
        };
        tracing::info!(mode = m, "clean stamps of copies (VICTORY_CLEAN_STAMP: 0 off, 1 on, 2 verify)");
        m
    })
}

/// Generation counters in `watch` are consumed only by clean stamps. When
/// `VICTORY_CLEAN_STAMP=0` and `VICTORY_SKIP_UNUSED_STAMPS=1`, skip `bump_block` work.
pub(crate) fn skip_unused_stamps() -> bool {
    static SKIP: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *SKIP.get_or_init(|| {
        let requested = std::env::var_os("VICTORY_SKIP_UNUSED_STAMPS").is_some_and(|v| v == "1");
        let mode = clean_stamp();
        let active = requested && mode == 0;
        tracing::info!(requested, active, clean_stamp = mode, "VICTORY_SKIP_UNUSED_STAMPS");
        active
    })
}

pub(crate) const S_HIT: usize = 0;
const S_LOOKS: usize = 1;
const S_LOOK_NS: usize = 2;
const S_STAMPED_LOOKS: usize = 3;
const S_MISS: usize = 4;
const S_CLEAN: usize = 5;
static STAMPS: [std::sync::atomic::AtomicU64; 6] = [const { std::sync::atomic::AtomicU64::new(0) }; 6];
static MISSES_ALL: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

pub(crate) fn stamp_count(i: usize, n: u64) {
    STAMPS[i].fetch_add(n, std::sync::atomic::Ordering::Relaxed);
}

/// A look of `nothing_new` that began at `at`: whether a stamp said clean, and what it found.
pub(crate) fn stamp_looked(stamped: bool, same: bool, at: std::time::Instant) {
    stamp_count(S_LOOKS, 1);
    stamp_count(S_LOOK_NS, at.elapsed().as_nanos() as u64);
    if same {
        stamp_count(S_CLEAN, 1);
    }
    if stamped {
        stamp_count(S_STAMPED_LOOKS, 1);
        if !same {
            stamp_count(S_MISS, 1);
            MISSES_ALL.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
    }
}

/// Per kind (`0` known clean, `1` refreshed): count, nanoseconds.
static FLUSHED: [std::sync::atomic::AtomicU64; 4] = [const { std::sync::atomic::AtomicU64::new(0) }; 4];

/// A refresh at a flush (`crate::host::Batch::flush_one`) that began at `at` came to `kind`.
pub(crate) fn flushed(kind: usize, at: std::time::Instant) {
    use std::sync::atomic::Ordering::Relaxed;
    if clean_stamp() == 0 {
        return;
    }
    let idx = kind.min(1) * 2;
    FLUSHED[idx].fetch_add(1, Relaxed);
    FLUSHED[idx + 1].fetch_add(at.elapsed().as_nanos() as u64, Relaxed);
    static LAST: Mutex<Option<(std::time::Instant, u64)>> = Mutex::new(None);
    let now = std::time::Instant::now();
    let frame = crate::frame::current();
    let frames = {
        let mut last = LAST.lock().unwrap_or_else(|e| e.into_inner());
        let (was, f) = *last.get_or_insert((now, frame));
        if now.duration_since(was).as_secs() < 5 {
            return;
        }
        *last = Some((now, frame));
        (frame.saturating_sub(f)).max(1) as f64
    };
    let f: [u64; 4] = std::array::from_fn(|i| FLUSHED[i].swap(0, Relaxed));
    let s: [u64; 6] = std::array::from_fn(|i| STAMPS[i].swap(0, Relaxed));
    let per = |n: u64| n as f64 / frames;
    let us_each = |ns: u64, n: u64| if n == 0 { 0.0 } else { ns as f64 / 1e3 / n as f64 };
    tracing::info!(
        "refreshes at flushes per frame (submitting thread): known clean {:.0} in {:.2} ms ({:.1} us each), \
         refreshed {:.0} in {:.2} ms ({:.1} us each); clean looks (VICTORY_CLEAN_STAMP={}): answered by the \
         stamp {:.0}, looked {:.0} in {:.2} ms ({:.1} us each, {:.0} found clean), of them stamped {:.0}; \
         MUST BE 0 since the start: stamped clean but the look found otherwise {}",
        per(f[0]),
        f[1] as f64 / 1e6 / frames,
        us_each(f[1], f[0]),
        per(f[2]),
        f[3] as f64 / 1e6 / frames,
        us_each(f[3], f[2]),
        ["0", "1", "verify"][clean_stamp() as usize],
        per(s[S_HIT]),
        per(s[S_LOOKS]),
        s[S_LOOK_NS] as f64 / 1e6 / frames,
        us_each(s[S_LOOK_NS], s[S_LOOKS]),
        per(s[S_CLEAN]),
        per(s[S_STAMPED_LOOKS]),
        MISSES_ALL.load(Relaxed),
    );
}
