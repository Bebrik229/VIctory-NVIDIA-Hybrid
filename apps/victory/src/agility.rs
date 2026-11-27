//! Diagnoses a missing application-local Agility SDK runtime.
//!
//! `D3D12SDKPath` in `main.rs` asks the Windows D3D12 loader for
//! `D3D12Core.dll` in a folder beside the executable.

use std::path::{Path, PathBuf};

/// The folder `D3D12SDKPath` names, relative to the executable.
const SDK_DIR: &str = "d3d12sdk";
const RUNTIME: &str = "D3D12Core.dll";

fn runtime_path(executable: &Path) -> PathBuf {
    executable.with_file_name(SDK_DIR).join(RUNTIME)
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

/// Warns before Direct3D starts when the runtime the executable requests is absent.
#[cfg(windows)]
pub fn warn_if_runtime_missing() {
    let Ok(executable) = std::env::current_exe() else { return };
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
}
