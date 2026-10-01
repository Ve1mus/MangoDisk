//! Use case: explain what is using physical memory right now.
//!
//! The tray overview answers "how full is memory". This answers "who is using
//! it": a breakdown by kind of memory plus every application's footprint,
//! grouped the way Activity Monitor groups a bundle with its helper processes.
//! Footprints can legitimately add up to more than installed RAM because they
//! include memory that was compressed or swapped out, so they are a ranking and
//! are never presented as a share of the whole.
use std::{
    collections::BTreeMap,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use mangodisk_platform::system_resources::memory_breakdown::{
    self, MemoryBreakdownSnapshot, MemoryPressureLevel, ProcessFootprint, ProcessMemoryMetric,
};
use serde::Serialize;

use crate::{applications::running_identity, CoreResult};

pub const MEMORY_ANALYSIS_SCHEMA_VERSION: u32 = 1;
/// Bounds the IPC payload and native icon requests while keeping a long, scrollable ranking.
const MAX_CONSUMERS: usize = 40;
/// Largest processes returned per application; `process_count` still covers all of them.
const MAX_PROCESSES_PER_CONSUMER: usize = 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum MemoryPressure {
    Normal,
    Warning,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum MemoryMetric {
    /// Resident, compressed, and swapped memory a process owns (Activity Monitor's "Memory").
    Footprint,
    /// Resident (working set) size only.
    Resident,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryCategoryBytes {
    pub application_bytes: u64,
    pub wired_bytes: u64,
    pub compressed_bytes: u64,
    pub cached_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryAnalysisOverview {
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub used_percent: u8,
    pub free_bytes: u64,
    pub swap_used_bytes: u64,
    pub swap_total_bytes: u64,
    /// Absent where the platform does not expose these counters.
    pub categories: Option<MemoryCategoryBytes>,
    pub pressure: Option<MemoryPressure>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryConsumerProcess {
    pub pid: u32,
    pub name: String,
    pub bytes: u64,
    pub approximate: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryConsumer {
    pub id: String,
    pub name: String,
    pub bytes: u64,
    pub process_count: u32,
    pub processes: Vec<MemoryConsumerProcess>,
    /// Bundle or executable location for native icons and file-manager navigation; never telemetry.
    pub icon_path: Option<String>,
    pub is_bundle: bool,
    pub can_quit: bool,
    /// At least one process could only be measured by resident size.
    pub approximate: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryAnalysis {
    pub schema_version: u32,
    pub sampled_at_ms: u64,
    pub metric: MemoryMetric,
    pub overview: MemoryAnalysisOverview,
    pub consumers: Vec<MemoryConsumer>,
    /// Processes that reported no memory and were left out of the ranking.
    pub omitted_process_count: u32,
    pub elapsed_ms: u64,
}

pub struct MemoryAnalysisService;

impl MemoryAnalysisService {
    pub fn analyze() -> CoreResult<MemoryAnalysis> {
        let started = Instant::now();
        let breakdown = memory_breakdown::sample_breakdown()?;
        let processes = memory_breakdown::sample_processes()?;
        let (consumers, omitted_process_count) =
            group_consumers(processes.processes, std::process::id());
        let sampled_at_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|elapsed| elapsed.as_millis() as u64)
            .unwrap_or(0);
        let elapsed_ms = started.elapsed().as_millis() as u64;
        log::info!(
            "memory_analysis_completed consumer_count={} omitted_process_count={omitted_process_count} elapsed_ms={elapsed_ms}",
            consumers.len()
        );
        Ok(MemoryAnalysis {
            schema_version: MEMORY_ANALYSIS_SCHEMA_VERSION,
            sampled_at_ms,
            metric: match processes.metric {
                ProcessMemoryMetric::Footprint => MemoryMetric::Footprint,
                ProcessMemoryMetric::Resident => MemoryMetric::Resident,
            },
            overview: overview(&breakdown),
            consumers,
            omitted_process_count,
            elapsed_ms,
        })
    }
}

fn overview(raw: &MemoryBreakdownSnapshot) -> MemoryAnalysisOverview {
    // Counters are read separately by the OS; bound transient inconsistencies.
    let used_bytes = raw.used_bytes.min(raw.total_bytes);
    MemoryAnalysisOverview {
        total_bytes: raw.total_bytes,
        used_bytes,
        used_percent: ((u128::from(used_bytes) * 100 + u128::from(raw.total_bytes) / 2)
            / u128::from(raw.total_bytes.max(1))) as u8,
        free_bytes: raw.free_bytes.min(raw.total_bytes),
        swap_used_bytes: raw.swap_used_bytes,
        swap_total_bytes: raw.swap_total_bytes,
        categories: raw.categories.map(|categories| MemoryCategoryBytes {
            application_bytes: categories.application_bytes,
            wired_bytes: categories.wired_bytes,
            compressed_bytes: categories.compressed_bytes,
            cached_bytes: categories.cached_bytes,
        }),
        pressure: raw.pressure.map(|level| match level {
            MemoryPressureLevel::Normal => MemoryPressure::Normal,
            MemoryPressureLevel::Warning => MemoryPressure::Warning,
            MemoryPressureLevel::Critical => MemoryPressure::Critical,
        }),
    }
}

fn group_consumers(
    processes: Vec<ProcessFootprint>,
    current_pid: u32,
) -> (Vec<MemoryConsumer>, u32) {
    // Resolve our own identity first so a helper row never offers to quit MangoDisk.
    let own_path = processes
        .iter()
        .find(|process| process.pid == current_pid)
        .and_then(|process| process.executable.as_deref())
        .map(running_identity::application_path);
    let mut groups = BTreeMap::<String, MemoryConsumer>::new();
    let mut omitted_process_count = 0;
    for process in processes {
        if process.bytes == 0 || process.name.trim().is_empty() {
            omitted_process_count += 1;
            continue;
        }
        let application_path = process
            .executable
            .as_deref()
            .map(running_identity::application_path);
        let is_bundle = application_path
            .as_deref()
            .is_some_and(running_identity::is_bundle);
        // The same identity the quit command resolves, so a row can be acted on later.
        let id = running_identity::id(application_path.as_deref(), process.pid);
        let consumer = groups.entry(id.clone()).or_insert_with(|| MemoryConsumer {
            id,
            name: application_path
                .as_deref()
                .filter(|_| is_bundle)
                .and_then(|path| path.file_stem())
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| process.name.clone()),
            bytes: 0,
            process_count: 0,
            processes: Vec::new(),
            icon_path: application_path
                .as_ref()
                .map(|path| path.to_string_lossy().into_owned()),
            is_bundle,
            can_quit: application_path
                .as_deref()
                .is_some_and(|path| running_identity::can_quit(path, own_path.as_deref())),
            approximate: false,
        });
        consumer.bytes = consumer.bytes.saturating_add(process.bytes);
        consumer.process_count += 1;
        consumer.approximate |= process.approximate;
        consumer.processes.push(MemoryConsumerProcess {
            pid: process.pid,
            name: process.name,
            bytes: process.bytes,
            approximate: process.approximate,
        });
    }
    let mut consumers = groups.into_values().collect::<Vec<_>>();
    for consumer in &mut consumers {
        consumer.processes.sort_by(|left, right| {
            right
                .bytes
                .cmp(&left.bytes)
                .then_with(|| left.pid.cmp(&right.pid))
        });
        consumer.processes.truncate(MAX_PROCESSES_PER_CONSUMER);
    }
    consumers.sort_by(|left, right| {
        right
            .bytes
            .cmp(&left.bytes)
            .then_with(|| left.name.cmp(&right.name))
            .then_with(|| left.id.cmp(&right.id))
    });
    consumers.truncate(MAX_CONSUMERS);
    (consumers, omitted_process_count)
}

#[cfg(test)]
mod tests {
    use mangodisk_platform::system_resources::memory_breakdown::MemoryCategories;

    use super::*;

    fn process(pid: u32, path: Option<&str>, bytes: u64) -> ProcessFootprint {
        ProcessFootprint {
            pid,
            name: format!("Process{pid}"),
            executable: path.map(Into::into),
            bytes,
            approximate: false,
        }
    }

    #[test]
    fn helpers_are_grouped_under_their_bundle_and_ranked_by_total_footprint() {
        let (consumers, omitted) = group_consumers(
            vec![
                process(1, Some("/Applications/Browser.app/Contents/MacOS/Browser"), 10),
                process(
                    2,
                    Some("/Applications/Browser.app/Contents/Frameworks/Helper.app/Contents/MacOS/Helper"),
                    40,
                ),
                process(3, Some("/Applications/Editor.app/Contents/MacOS/Editor"), 30),
                process(4, None, 0),
            ],
            u32::MAX,
        );

        assert_eq!(omitted, 1);
        assert_eq!(consumers.len(), 2);
        assert_eq!(consumers[0].name, "Browser");
        assert_eq!(consumers[0].bytes, 50);
        assert_eq!(consumers[0].process_count, 2);
        assert_eq!(consumers[0].processes[0].pid, 2, "largest process first");
        assert!(consumers[0].is_bundle && consumers[0].can_quit);
    }

    #[test]
    fn own_bundle_and_unbundled_processes_cannot_be_quit() {
        let (consumers, _) = group_consumers(
            vec![
                process(1, Some("/Apps/MangoDisk.app/Contents/MacOS/MangoDisk"), 10),
                process(2, Some("/usr/bin/node"), 9),
                process(3, None, 8),
            ],
            1,
        );

        assert_eq!(consumers.len(), 3);
        assert!(consumers.iter().all(|consumer| !consumer.can_quit));
    }

    #[test]
    fn an_approximate_process_marks_its_whole_application() {
        let mut denied = process(2, Some("/Apps/Tool.app/Contents/MacOS/Tool"), 5);
        denied.approximate = true;
        let (consumers, _) = group_consumers(
            vec![
                process(1, Some("/Apps/Tool.app/Contents/MacOS/Helper"), 5),
                denied,
            ],
            u32::MAX,
        );

        assert!(consumers[0].approximate);
        assert!(consumers[0].processes.iter().any(|row| row.approximate));
    }

    #[test]
    fn ranking_is_bounded_and_independent_of_input_order() {
        let inputs = (1..=60)
            .map(|pid| process(pid, None, u64::from(pid)))
            .collect::<Vec<_>>();
        let forward = group_consumers(inputs.clone(), u32::MAX).0;
        let reverse = group_consumers(inputs.into_iter().rev().collect(), u32::MAX).0;

        assert_eq!(forward.len(), MAX_CONSUMERS);
        assert_eq!(forward[0].bytes, 60);
        assert_eq!(forward, reverse);
    }

    #[test]
    fn per_application_process_list_is_bounded_but_counted() {
        let inputs = (1..=30)
            .map(|pid| {
                process(
                    pid,
                    Some("/Apps/Many.app/Contents/MacOS/Many"),
                    u64::from(pid),
                )
            })
            .collect();

        let (consumers, _) = group_consumers(inputs, u32::MAX);

        assert_eq!(consumers[0].process_count, 30);
        assert_eq!(consumers[0].processes.len(), MAX_PROCESSES_PER_CONSUMER);
        assert_eq!(consumers[0].bytes, (1..=30).sum::<u64>());
    }

    #[test]
    fn overview_bounds_racing_counters_and_keeps_categories() {
        let raw = MemoryBreakdownSnapshot {
            total_bytes: 100,
            used_bytes: 130,
            free_bytes: 250,
            swap_used_bytes: 7,
            swap_total_bytes: 20,
            categories: Some(MemoryCategories {
                application_bytes: 40,
                wired_bytes: 20,
                compressed_bytes: 30,
                cached_bytes: 10,
            }),
            pressure: Some(MemoryPressureLevel::Warning),
        };

        let result = overview(&raw);
        let json = serde_json::to_value(&result).unwrap();

        assert_eq!(result.used_percent, 100);
        assert_eq!(result.free_bytes, 100);
        assert_eq!(json["categories"]["compressedBytes"], 30);
        assert_eq!(json["pressure"], "warning");
    }

    #[test]
    fn analysis_of_this_machine_reports_this_process() {
        let analysis = MemoryAnalysisService::analyze().unwrap();

        assert!(analysis.overview.total_bytes > 0);
        assert!(!analysis.consumers.is_empty());
        assert_eq!(analysis.schema_version, MEMORY_ANALYSIS_SCHEMA_VERSION);
    }

    #[test]
    #[ignore = "prints the analysis of the real machine"]
    fn prints_the_installed_machine() {
        let analysis = MemoryAnalysisService::analyze().unwrap();
        println!(
            "{}",
            serde_json::to_string_pretty(&analysis.overview).unwrap()
        );
        println!(
            "metric={:?} elapsed_ms={}",
            analysis.metric, analysis.elapsed_ms
        );
        for consumer in analysis.consumers.iter().take(12) {
            println!(
                "{:>8} MB  {} ({} processes, approximate={})",
                consumer.bytes / 1_048_576,
                consumer.name,
                consumer.process_count,
                consumer.approximate
            );
        }
    }
}
