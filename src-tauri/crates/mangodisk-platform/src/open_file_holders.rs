//! Processes that hold a file, folder, or volume open, or every process with any open file.
//!
//! macOS uses `lsof`, which reports only what the current user may inspect. A
//! missing process is therefore not proof that nothing else holds the path.
use std::path::{Path, PathBuf};

use crate::{PlatformError, PlatformResult};

/// Open-file names kept per process; the count still covers every match.
pub const MAX_SAMPLE_PATHS: usize = 20;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenFileHolder {
    pub pid: i32,
    pub command: String,
    pub executable: Option<PathBuf>,
    pub open_file_count: u64,
    pub sample_paths: Vec<String>,
}

#[cfg(target_os = "macos")]
pub fn find(path: &Path) -> PlatformResult<Vec<OpenFileHolder>> {
    let metadata = std::fs::symlink_metadata(path).map_err(|error| {
        PlatformError::invalid_path(format!("open_file_holders_path_unreadable {error}"))
    })?;
    let target = path.to_string_lossy();
    // `+D` walks a directory tree; a single file needs a plain name argument.
    let arguments: Vec<&str> = if metadata.is_dir() {
        vec!["-nP", "-w", "-F", "pcn", "+D", target.as_ref()]
    } else {
        vec!["-nP", "-w", "-F", "pcn", "--", target.as_ref()]
    };
    run_lsof(&arguments, LsofLimits::PATH)
}

/// Every process of the current account with at least one open file or folder.
///
/// Memory-mapped libraries, executables, and working directories are left out: each
/// process maps hundreds of them, so they would bury the files a person recognizes.
/// Sockets, pipes, and devices are left out for the same reason.
#[cfg(target_os = "macos")]
pub fn find_all() -> PlatformResult<Vec<OpenFileHolder>> {
    run_lsof(
        &["-nP", "-w", "-F", "pcnt", "-d", "^mem,^txt,^cwd,^rtd"],
        LsofLimits::SYSTEM,
    )
}

#[cfg(target_os = "macos")]
struct LsofLimits {
    timeout_secs: u64,
    stdout_bytes: usize,
}

#[cfg(target_os = "macos")]
impl LsofLimits {
    const PATH: Self = Self {
        timeout_secs: 20,
        stdout_bytes: 8 * 1024 * 1024,
    };
    // A whole-system listing is a few megabytes on a busy desktop; leave headroom.
    const SYSTEM: Self = Self {
        timeout_secs: 60,
        stdout_bytes: 64 * 1024 * 1024,
    };
}

#[cfg(target_os = "macos")]
fn run_lsof(arguments: &[&str], limits: LsofLimits) -> PlatformResult<Vec<OpenFileHolder>> {
    use std::time::Duration;

    use crate::{
        run_controlled_command, ControlledCommandError, ControlledCommandLimits,
        ControlledEnvironmentPolicy, ControlledExecutable, PlatformFailureReason,
    };

    let executable =
        ControlledExecutable::capture(Path::new("/usr/sbin/lsof")).map_err(|error| {
            PlatformError::operation_failed(format!(
                "open_file_holders_tool_invalid reason={}",
                error.as_str()
            ))
            .with_failure_reason(PlatformFailureReason::ToolUnavailable)
        })?;
    let output = run_controlled_command(
        "macos-open-file-holders",
        &executable,
        arguments,
        ControlledEnvironmentPolicy::Inherit,
        ControlledCommandLimits {
            timeout: Duration::from_secs(limits.timeout_secs),
            stdout_bytes: limits.stdout_bytes,
            stderr_bytes: 64 * 1024,
        },
        &|| false,
    )
    .map_err(|error| {
        let reason = match error {
            ControlledCommandError::TimedOut => PlatformFailureReason::TimedOut,
            _ => PlatformFailureReason::ServiceUnavailable,
        };
        PlatformError::operation_failed(format!(
            "open_file_holders_command_failed reason={}",
            error.as_str()
        ))
        .with_failure_reason(reason)
    })?;
    // `lsof` exits with 1 when nothing matches, which is a valid empty answer.
    if !output.status.success() && !output.stdout.is_empty() {
        log::debug!("open_file_holders_partial_output status={}", output.status);
    }
    let mut holders = parse(&String::from_utf8_lossy(&output.stdout));
    let launch_paths =
        launch_executable_paths(&holders.iter().map(|holder| holder.pid).collect::<Vec<_>>());
    for holder in &mut holders {
        holder.executable = launch_paths.get(&holder.pid).cloned();
    }
    Ok(holders)
}

#[cfg(not(target_os = "macos"))]
pub fn find(_path: &Path) -> PlatformResult<Vec<OpenFileHolder>> {
    Err(unsupported())
}

#[cfg(not(target_os = "macos"))]
pub fn find_all() -> PlatformResult<Vec<OpenFileHolder>> {
    Err(unsupported())
}

#[cfg(not(target_os = "macos"))]
fn unsupported() -> PlatformError {
    PlatformError::new(
        crate::PlatformErrorCode::Unsupported,
        "open file holders are not supported on this platform",
    )
}

/// Executable paths as `ps` reports them, which is the identity the process-close
/// adapter matches. `proc_pidpath` can differ for a process started from an
/// updater staging copy that was later removed, so it is not used here.
#[cfg(target_os = "macos")]
fn launch_executable_paths(pids: &[i32]) -> std::collections::HashMap<i32, PathBuf> {
    use std::time::Duration;

    use crate::{
        run_controlled_command, ControlledCommandLimits, ControlledEnvironmentPolicy,
        ControlledExecutable,
    };

    if pids.is_empty() {
        return std::collections::HashMap::new();
    }
    let Ok(executable) = ControlledExecutable::capture(Path::new("/bin/ps")) else {
        return std::collections::HashMap::new();
    };
    let list = pids
        .iter()
        .map(i32::to_string)
        .collect::<Vec<_>>()
        .join(",");
    let output = run_controlled_command(
        "macos-open-file-holder-paths",
        &executable,
        &["-o", "pid=,comm=", "-p", list.as_str()],
        ControlledEnvironmentPolicy::Inherit,
        ControlledCommandLimits {
            timeout: Duration::from_secs(5),
            stdout_bytes: 1024 * 1024,
            stderr_bytes: 64 * 1024,
        },
        &|| false,
    );
    output
        .map(|output| parse_process_paths(&String::from_utf8_lossy(&output.stdout)))
        .unwrap_or_default()
}

#[cfg(any(target_os = "macos", test))]
fn parse_process_paths(output: &str) -> std::collections::HashMap<i32, PathBuf> {
    output
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            let (pid, path) = line.split_once(char::is_whitespace)?;
            let path = path.trim();
            (!path.is_empty()).then(|| Some((pid.parse::<i32>().ok()?, PathBuf::from(path))))?
        })
        .collect()
}

/// Parses `lsof -F pcn[t]` output: `p` starts a process, `c` names its command, and each
/// `n` line is one open file. When the type field `t` is present, only regular files and
/// directories count. Other field lines are ignored.
#[cfg(any(target_os = "macos", test))]
fn parse(output: &str) -> Vec<OpenFileHolder> {
    let mut holders: Vec<OpenFileHolder> = Vec::new();
    // Each file descriptor is reported as `f`, then optionally `t`, then `n`.
    let mut counts_as_file = true;
    for line in output.lines() {
        let Some(tag) = line.chars().next() else {
            continue;
        };
        let value = &line[tag.len_utf8()..];
        match tag {
            'p' => {
                if let Ok(pid) = value.parse::<i32>() {
                    holders.push(OpenFileHolder {
                        pid,
                        command: String::new(),
                        executable: None,
                        open_file_count: 0,
                        sample_paths: Vec::new(),
                    });
                }
            }
            'c' => {
                if let Some(holder) = holders.last_mut() {
                    holder.command = value.to_string();
                }
            }
            'f' => counts_as_file = true,
            't' => counts_as_file = matches!(value, "REG" | "DIR"),
            'n' => {
                if !counts_as_file {
                    continue;
                }
                if let Some(holder) = holders.last_mut() {
                    holder.open_file_count += 1;
                    if holder.sample_paths.len() < MAX_SAMPLE_PATHS
                        && !holder.sample_paths.iter().any(|path| path == value)
                    {
                        holder.sample_paths.push(value.to_string());
                    }
                }
            }
            _ => {}
        }
    }
    holders.retain(|holder| holder.open_file_count > 0);
    holders.sort_by(|left, right| {
        right
            .open_file_count
            .cmp(&left.open_file_count)
            .then(left.pid.cmp(&right.pid))
    });
    holders
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn groups_open_files_by_process_and_keeps_a_bounded_sample() {
        let output = "p10\ncServiceExtension\nf3\nn/a/db.sqlite\nf4\nn/a/db.sqlite-wal\nf5\nn/a/db.sqlite\np20\ncOther\nf9\nn/a/x\n";

        let holders = parse(output);

        assert_eq!(holders.len(), 2);
        assert_eq!(holders[0].pid, 10);
        assert_eq!(holders[0].command, "ServiceExtension");
        assert_eq!(holders[0].open_file_count, 3);
        assert_eq!(
            holders[0].sample_paths,
            ["/a/db.sqlite", "/a/db.sqlite-wal"]
        );
        assert_eq!(holders[1].open_file_count, 1);
    }

    #[test]
    fn typed_listings_count_only_regular_files_and_directories() {
        let output = "p10\ncApp\nf3\ntREG\nn/a/file\nf4\ntunix\nn->0x2ec4\nf5\ntCHR\nn/dev/null\nf6\ntDIR\nn/a/dir\np20\ncSocketsOnly\nf1\ntIPv4\nn*:80\n";

        let holders = parse(output);

        assert_eq!(
            holders.len(),
            1,
            "a process holding only sockets is not listed"
        );
        assert_eq!(holders[0].open_file_count, 2);
        assert_eq!(holders[0].sample_paths, ["/a/file", "/a/dir"]);
    }

    #[test]
    fn ignores_processes_without_matching_files_and_malformed_lines() {
        let holders = parse("p10\ncIdle\np11\ncBusy\nn/a/b\nnot-a-field\n\nPx\np\n");

        assert_eq!(holders.len(), 1);
        assert_eq!(holders[0].pid, 11);
    }

    #[test]
    fn limits_sample_paths_but_counts_every_open_file() {
        let mut output = String::from("p1\ncBusy\n");
        for index in 0..20 {
            output.push_str(&format!("n/a/{index}\n"));
        }

        let holders = parse(&output);

        assert_eq!(holders[0].open_file_count, 20);
        assert_eq!(holders[0].sample_paths.len(), MAX_SAMPLE_PATHS);
    }

    #[test]
    fn process_paths_keep_spaces_and_skip_malformed_lines() {
        let paths = parse_process_paths(
            "  42 /Applications/WPS Office.app/Contents/MacOS/wpsoffice\nnot-a-line\n 7\n",
        );

        assert_eq!(paths.len(), 1);
        assert_eq!(
            paths[&42],
            PathBuf::from("/Applications/WPS Office.app/Contents/MacOS/wpsoffice")
        );
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn the_system_listing_includes_a_file_this_process_holds() {
        let directory = tempfile::tempdir().unwrap();
        let file = directory.path().join("held-system-wide.txt");
        let handle = std::fs::File::create(&file).unwrap();

        let holders = find_all().unwrap();
        drop(handle);

        let own = holders
            .iter()
            .find(|holder| holder.pid == std::process::id() as i32)
            .expect("the test process is listed");
        assert!(own.open_file_count >= 1);
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn finds_this_process_holding_its_own_executable_directory() {
        let directory = tempfile::tempdir().unwrap();
        let file = directory.path().join("held.txt");
        let handle = std::fs::File::create(&file).unwrap();

        let holders = find(directory.path()).unwrap();
        drop(handle);

        assert!(holders
            .iter()
            .any(|holder| holder.pid == std::process::id() as i32));
    }
}
