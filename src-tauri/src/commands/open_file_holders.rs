use mangodisk_core::{
    ApplicationCloseBatchResult, ApplicationCloseMode, OpenFileHolderService, OpenFileHoldersResult,
};

use super::error::{run_blocking, CommandResult};

#[tauri::command]
pub async fn find_open_file_holders(path: Option<String>) -> CommandResult<OpenFileHoldersResult> {
    run_blocking("find_open_file_holders", move || {
        OpenFileHolderService::search(path)
    })
    .await
}

#[tauri::command]
pub async fn close_open_file_holders(
    path: Option<String>,
    executable_paths: Vec<String>,
    mode: ApplicationCloseMode,
) -> CommandResult<ApplicationCloseBatchResult> {
    run_blocking("close_open_file_holders", move || {
        OpenFileHolderService::close(path, executable_paths, mode)
    })
    .await
}
