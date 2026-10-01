//! Detailed memory accounting for the question "what is using my RAM".
//!
//! `memory` keeps the cheap overview sampled by the tray. This module is sampled
//! on demand and adds what an overview cannot show: where physical memory goes
//! (application, wired, compressed, cached) and each process's real footprint.
//! macOS compresses and swaps inactive application memory, so resident size
//! alone can account for only a fraction of the memory a process really costs.
use std::path::PathBuf;

use sysinfo::{MemoryRefreshKind, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

use crate::{PlatformError, PlatformErrorCode, PlatformResult};

#[cfg(target_os = "macos")]
#[path = "memory_breakdown/macos.rs"]
mod native;

/// Other platforms report resident (working set) size and expose no extra counters.
#[cfg(not(target_os = "macos"))]
mod native {
    use super::{ProcessFootprint, ProcessFootprintSnapshot, ProcessMemoryMetric};

    pub(super) fn footprints(processes: Vec<ProcessFootprint>) -> ProcessFootprintSnapshot {
        ProcessFootprintSnapshot {
            metric: ProcessMemoryMetric::Resident,
            processes,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryPressureLevel {
    Normal,
    Warning,
    Critical,
}

/// Where used physical memory goes, using Activity Monitor's definitions.
/// `used = application + wired + compressed`; cached files are reclaimable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryCategories {
    pub application_bytes: u64,
    pub wired_bytes: u64,
    pub compressed_bytes: u64,
    pub cached_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryBreakdownSnapshot {
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub free_bytes: u64,
    pub swap_used_bytes: u64,
    pub swap_total_bytes: u64,
    /// `None` when the platform does not expose these counters.
    pub categories: Option<MemoryCategories>,
    pub pressure: Option<MemoryPressureLevel>,
}

/// What `ProcessFootprint::bytes` measures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessMemoryMetric {
    /// Physical footprint: resident, compressed, and swapped memory the process owns.
    Footprint,
    /// Resident (working set) size only.
    Resident,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessFootprint {
    pub pid: u32,
    pub name: String,
    pub executable: Option<PathBuf>,
    pub bytes: u64,
    /// The preferred metric was denied for this process and resident size stands in.
    pub approximate: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessFootprintSnapshot {
    pub metric: ProcessMemoryMetric,
    pub processes: Vec<ProcessFootprint>,
}

pub fn sample_breakdown() -> PlatformResult<MemoryBreakdownSnapshot> {
    let mut system = System::new();
    system.refresh_memory_specifics(MemoryRefreshKind::everything());
    if system.total_memory() == 0 {
        return Err(PlatformError::new(
            PlatformErrorCode::OperationFailed,
            "system memory counters are unavailable",
        ));
    }
    let total_bytes = system.total_memory();
    let mut snapshot = MemoryBreakdownSnapshot {
        total_bytes,
        used_bytes: system.used_memory().min(total_bytes),
        free_bytes: system.free_memory().min(total_bytes),
        swap_used_bytes: system.used_swap(),
        swap_total_bytes: system.total_swap(),
        categories: None,
        pressure: None,
    };
    #[cfg(target_os = "macos")]
    native::refine(&mut snapshot);
    Ok(snapshot)
}

pub fn sample_processes() -> PlatformResult<ProcessFootprintSnapshot> {
    let mut system = System::new();
    // Names and image paths only: never request environment variables or command lines.
    system.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing()
            .with_memory()
            .with_exe(UpdateKind::OnlyIfNotSet),
    );
    let processes = system
        .processes()
        .iter()
        .map(|(pid, process)| ProcessFootprint {
            pid: pid.as_u32(),
            name: process.name().to_string_lossy().into_owned(),
            executable: process.exe().map(PathBuf::from),
            bytes: process.memory(),
            approximate: false,
        })
        .collect();
    Ok(native::footprints(processes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn breakdown_is_consistent_with_total_capacity() {
        let snapshot = sample_breakdown().expect("memory counters should be readable");
        assert!(snapshot.total_bytes > 0);
        assert!(snapshot.used_bytes <= snapshot.total_bytes);
        if let Some(categories) = snapshot.categories {
            // The categories add up to the reported use, so the page cannot contradict itself.
            let categorized =
                categories.application_bytes + categories.wired_bytes + categories.compressed_bytes;
            assert_eq!(categorized.min(snapshot.total_bytes), snapshot.used_bytes);
        }
    }

    #[test]
    fn this_process_has_a_nonzero_footprint() {
        let snapshot = sample_processes().expect("process sampling should complete");
        assert!(snapshot
            .processes
            .iter()
            .any(|process| process.pid == std::process::id() && process.bytes > 0));
    }
}
