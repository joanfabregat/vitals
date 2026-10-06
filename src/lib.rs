//! vitals: one-line Linux system metrics for status bars.

pub mod cli;
pub mod clock;
pub mod cpu;
pub mod disk;
pub mod mem;
pub mod render;

use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;

use cli::Options;
use render::{Readings, Renderer};

const GIB: u64 = 1 << 30;

/// Where the CPU sample lives between runs: the per-user runtime directory
/// when there is one, else a per-user file in the temporary directory.
pub fn default_state_path() -> PathBuf {
    match std::env::var_os("XDG_RUNTIME_DIR").filter(|d| !d.is_empty()) {
        Some(dir) => PathBuf::from(dir).join("vitals-cpu"),
        None => {
            let uid = std::fs::metadata("/proc/self")
                .map(|m| m.uid())
                .unwrap_or(0);
            std::env::temp_dir().join(format!("vitals-cpu.{uid}"))
        }
    }
}

/// Takes every reading and renders the line. A reading that fails is left
/// out rather than failing the whole line.
pub fn line(options: &Options) -> String {
    let state = options.state.clone().unwrap_or_else(default_state_path);
    let memory = mem::read();
    let readings = Readings {
        cpu: cpu::read(&state),
        memory: memory.map(|m| m.memory),
        swap: memory.map(|m| m.swap),
        disks: disk::read(options.min_disk_gib.saturating_mul(GIB)),
        clock: options
            .clock
            .then(clock::now)
            .flatten()
            .map(|t| (t.date(), t.time())),
    };
    Renderer {
        style: options.style,
        colors: &options.colors,
        alert: options.alert,
    }
    .render(&readings)
}
