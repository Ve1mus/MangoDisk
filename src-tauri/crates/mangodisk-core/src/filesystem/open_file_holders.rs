//! Use case: which processes keep a path open, or which applications hold any file open.
use std::{path::Path, time::Instant};

use mangodisk_platform::open_file_holders;
use serde::Serialize;

use crate::{
    applications::{
        process_control::{
            close_resolved_applications, ApplicationCloseBatchResult, ApplicationCloseMode,
            ResolvedApplicationCloseTarget,
        },
        running_identity,
    },
    filesystem::metadata::diagnostic_path,
    CoreError, CoreResult,
};

pub const OPEN_FILE_HOLDERS_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenFileHolder {
    pub pid: i32,
    pub command: String,
    pub executable_path: Option<String>,
    /// The owning `.app` bundle when the process runs from one.
    pub application_path: Option<String>,
    pub open_file_count: u64,
    pub sample_paths: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenFileHoldersResult {
    pub schema_version: u32,
    /// The path that was searched; `None` lists every open file of the account.
    pub path: Option<String>,
    pub holders: Vec<OpenFileHolder>,
    pub elapsed_ms: u64,
}

pub struct OpenFileHolderService;

impl OpenFileHolderService {
    /// Closes the processes behind some holders of `path`, or of any file when `path` is `None`.
    ///
    /// Every executable must still be a current holder, so this command cannot be pointed
    /// at an arbitrary process. A graceful request lets the application save; force is an
    /// explicit second step.
    pub fn close(
        path: Option<String>,
        executable_paths: Vec<String>,
        mode: ApplicationCloseMode,
    ) -> CoreResult<ApplicationCloseBatchResult> {
        if executable_paths.is_empty() {
            return Err(CoreError::invalid_input("no process was selected"));
        }
        let holders = Self::search(path)?.holders;
        let all_hold = executable_paths.iter().all(|executable| {
            holders
                .iter()
                .any(|holder| holder.executable_path.as_deref() == Some(executable.as_str()))
        });
        if !all_hold {
            return Err(CoreError::operation_failed(
                "the process no longer holds an open file",
            ));
        }
        close_resolved_applications(
            vec![ResolvedApplicationCloseTarget {
                target_id: "open-file-holder".to_string(),
                executable_names: Vec::new(),
                executable_paths: executable_paths.into_iter().map(Into::into).collect(),
            }],
            mode,
        )
    }

    /// Lists the holders of `path`, or of every file when `path` is `None`.
    pub fn search(path: Option<String>) -> CoreResult<OpenFileHoldersResult> {
        let started = Instant::now();
        let raw = match path.as_deref() {
            Some(path) => {
                let target = Path::new(path);
                if !target.is_absolute() {
                    return Err(CoreError::invalid_input("path must be absolute"));
                }
                open_file_holders::find(target)?
            }
            None => open_file_holders::find_all()?,
        };
        let holders = raw
            .into_iter()
            .map(|holder| {
                let application_path = holder
                    .executable
                    .as_deref()
                    .map(running_identity::application_path)
                    .filter(|path| running_identity::is_bundle(path));
                OpenFileHolder {
                    pid: holder.pid,
                    command: holder.command,
                    executable_path: holder
                        .executable
                        .map(|path| path.to_string_lossy().into_owned()),
                    application_path: application_path
                        .map(|path| path.to_string_lossy().into_owned()),
                    open_file_count: holder.open_file_count,
                    sample_paths: holder.sample_paths,
                }
            })
            .collect::<Vec<_>>();
        let elapsed_ms = started.elapsed().as_millis() as u64;
        log::info!(
            "open_file_holders_found path={} holder_count={} elapsed_ms={elapsed_ms}",
            path.as_deref()
                .map(|path| diagnostic_path(Path::new(path)))
                .unwrap_or_else(|| "<all>".to_string()),
            holders.len()
        );
        Ok(OpenFileHoldersResult {
            schema_version: OPEN_FILE_HOLDERS_SCHEMA_VERSION,
            path,
            holders,
            elapsed_ms,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_relative_paths_before_running_any_tool() {
        assert!(OpenFileHolderService::search(Some("relative/path".to_string())).is_err());
    }

    #[test]
    fn closing_requires_a_selected_process() {
        let error = OpenFileHolderService::close(None, Vec::new(), ApplicationCloseMode::Graceful)
            .expect_err("an empty selection must not reach the close adapter");

        assert!(error.diagnostic().contains("no process"));
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn refuses_to_close_a_process_that_does_not_hold_the_path() {
        let directory = tempfile::tempdir().unwrap();

        let error = OpenFileHolderService::close(
            Some(directory.path().to_string_lossy().into_owned()),
            vec!["/bin/sleep".to_string()],
            ApplicationCloseMode::Force,
        )
        .expect_err("an unrelated executable must not be closable through this path");

        assert!(error.diagnostic().contains("no longer holds"));
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn names_the_application_bundle_of_a_holder_when_it_runs_from_one() {
        let directory = tempfile::tempdir().unwrap();
        let _held = std::fs::File::create(directory.path().join("held")).unwrap();

        let result =
            OpenFileHolderService::search(Some(directory.path().to_string_lossy().into_owned()))
                .unwrap();

        let own = result
            .holders
            .iter()
            .find(|holder| holder.pid == std::process::id() as i32)
            .expect("the test process holds the file");
        assert!(own.open_file_count >= 1);
        assert!(own.sample_paths[0].ends_with("/held"));
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn the_system_listing_has_no_path_and_includes_this_process() {
        let directory = tempfile::tempdir().unwrap();
        let _held = std::fs::File::create(directory.path().join("held")).unwrap();

        let result = OpenFileHolderService::search(None).unwrap();

        assert!(result.path.is_none());
        assert!(result
            .holders
            .iter()
            .any(|holder| holder.pid == std::process::id() as i32));
    }
}
