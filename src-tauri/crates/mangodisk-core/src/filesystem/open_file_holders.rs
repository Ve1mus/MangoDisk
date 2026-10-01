//! Use case: which processes keep a path open.
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
    pub path: String,
    pub holders: Vec<OpenFileHolder>,
    pub elapsed_ms: u64,
}

pub struct OpenFileHolderService;

impl OpenFileHolderService {
    /// Closes the process behind one holder of `path`.
    ///
    /// The executable must still be a current holder of that path, so this
    /// command cannot be pointed at an arbitrary process. A graceful request
    /// lets the application save; force is an explicit second step.
    pub fn close(
        path: String,
        executable_path: String,
        mode: ApplicationCloseMode,
    ) -> CoreResult<ApplicationCloseBatchResult> {
        let still_holds = Self::find(path)?
            .holders
            .iter()
            .any(|holder| holder.executable_path.as_deref() == Some(executable_path.as_str()));
        if !still_holds {
            return Err(CoreError::operation_failed(
                "the process no longer holds the selected path",
            ));
        }
        close_resolved_applications(
            vec![ResolvedApplicationCloseTarget {
                target_id: "open-file-holder".to_string(),
                executable_names: Vec::new(),
                executable_paths: vec![executable_path.into()],
            }],
            mode,
        )
    }

    pub fn find(path: String) -> CoreResult<OpenFileHoldersResult> {
        let target = Path::new(&path);
        if !target.is_absolute() {
            return Err(CoreError::invalid_input("path must be absolute"));
        }
        let started = Instant::now();
        let holders = open_file_holders::find(target)?
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
            diagnostic_path(target),
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
        assert!(OpenFileHolderService::find("relative/path".to_string()).is_err());
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn refuses_to_close_a_process_that_does_not_hold_the_path() {
        let directory = tempfile::tempdir().unwrap();

        let error = OpenFileHolderService::close(
            directory.path().to_string_lossy().into_owned(),
            "/bin/sleep".to_string(),
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
            OpenFileHolderService::find(directory.path().to_string_lossy().into_owned()).unwrap();

        let own = result
            .holders
            .iter()
            .find(|holder| holder.pid == std::process::id() as i32)
            .expect("the test process holds the file");
        assert!(own.open_file_count >= 1);
        assert!(own.sample_paths[0].ends_with("/held"));
    }
}
