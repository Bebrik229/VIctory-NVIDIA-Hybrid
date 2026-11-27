//! Diagnoses a missing application-local Agility SDK runtime.
//!
//! `D3D12SDKPath` in `main.rs` asks the Windows D3D12 loader for
//! `D3D12Core.dll` in a folder beside the executable.

use std::path::{Path, PathBuf};

/// The folder `D3D12SDKPath` names, relative to the executable.
const SDK_DIR: &str = "D3D12";
const LEGACY_SDK_DIR: &str = "d3d12sdk";
const RUNTIME: &str = "D3D12Core.dll";
const DEBUG_LAYERS: &str = "d3d12SDKLayers.dll";

fn runtime_path(executable: &Path) -> PathBuf {
    executable.with_file_name(SDK_DIR).join(RUNTIME)
}

/// Copies a legacy `d3d12sdk\` runtime beside `executable` into `D3D12\` if `D3D12\D3D12Core.dll` is absent.
#[cfg(any(windows, test))]
fn migrate_legacy_runtime(executable: &Path) -> bool {
    let runtime = runtime_path(executable);
    if runtime.is_file() {
        return false;
    }
    let legacy_dir = executable.with_file_name(LEGACY_SDK_DIR);
    let legacy_runtime = legacy_dir.join(RUNTIME);
    if !legacy_runtime.is_file() {
        return false;
    }
    let Some(dest_dir) = runtime.parent() else {
        return false;
    };
    if std::fs::create_dir_all(dest_dir).is_err() {
        return false;
    }
    if std::fs::copy(&legacy_runtime, &runtime).is_err() {
        return false;
    }
    let legacy_layers = legacy_dir.join(DEBUG_LAYERS);
    if legacy_layers.is_file() {
        let _ = std::fs::copy(&legacy_layers, dest_dir.join(DEBUG_LAYERS));
    }
    true
}

fn missing_runtime(executable: &Path, version: u32) -> Option<String> {
    let runtime = runtime_path(executable);
    (!runtime.is_file()).then(|| {
        format!(
            "VIctory: Agility SDK {version} runtime not found at {}; copy D3D12Core.dll there \
             (see Agility SDK runtime in README.md)",
            runtime.display()
        )
    })
}

/// Ensures fallback migration from `d3d12sdk\` if needed, then warns before Direct3D starts when absent.
#[cfg(windows)]
pub fn warn_if_runtime_missing() {
    let Ok(executable) = std::env::current_exe() else { return };
    let _ = migrate_legacy_runtime(&executable);
    if let Some(warning) = missing_runtime(&executable, crate::D3D12SDKVersion) {
        eprintln!("{warning}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    #[test]
    fn the_checked_folder_is_the_exported_one() {
        let exported = std::str::from_utf8(crate::D3D12SDKPath).unwrap();
        assert_eq!(exported, format!(".\\{SDK_DIR}\\\0"));
    }

    #[test]
    fn a_missing_runtime_is_named_and_a_present_one_is_not() {
        let dir = std::env::temp_dir().join(format!("victory-agility-{}", std::process::id()));
        let executable = dir.join("victory.exe");
        let runtime = runtime_path(&executable);
        assert_eq!(runtime, dir.join(SDK_DIR).join(RUNTIME));
        let _ = std::fs::remove_dir_all(&dir);

        let warning = missing_runtime(&executable, 619).expect("no runtime yet");
        assert!(warning.contains(&runtime.display().to_string()), "{warning}");
        assert!(warning.contains("619"), "{warning}");

        std::fs::create_dir_all(runtime.parent().unwrap()).unwrap();
        std::fs::write(&runtime, b"").unwrap();
        assert_eq!(missing_runtime(&executable, 619), None);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn legacy_d3d12sdk_folder_is_copied_to_d3d12_when_missing() {
        let dir =
            std::env::temp_dir().join(format!("victory-agility-legacy-{}", std::process::id()));
        let executable = dir.join("victory.exe");
        let legacy_dir = dir.join(LEGACY_SDK_DIR);
        let dest_dir = dir.join(SDK_DIR);
        let _ = std::fs::remove_dir_all(&dir);

        std::fs::create_dir_all(&legacy_dir).unwrap();
        std::fs::write(legacy_dir.join(RUNTIME), b"core").unwrap();
        std::fs::write(legacy_dir.join(DEBUG_LAYERS), b"layers").unwrap();

        assert!(migrate_legacy_runtime(&executable));
        assert!(dest_dir.join(RUNTIME).is_file());
        assert!(dest_dir.join(DEBUG_LAYERS).is_file());
        assert_eq!(missing_runtime(&executable, 619), None);
        assert!(!migrate_legacy_runtime(&executable));

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
