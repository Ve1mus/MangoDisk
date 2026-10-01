use mangodisk_core::{
    ApplicationCloseBatchResult, ApplicationCloseMode, OpenFileHolderService, OpenFileHoldersResult,
};

use super::error::{run_blocking, CommandResult};

#[tauri::command]
pub async fn find_open_file_holders(path: String) -> CommandResult<OpenFileHoldersResult> {
    run_blocking("find_open_file_holders", move || {
        OpenFileHolderService::find(path)
    })
    .await
}

#[tauri::command]
pub async fn close_open_file_holder(
    path: String,
    executable_path: String,
    mode: ApplicationCloseMode,
) -> CommandResult<ApplicationCloseBatchResult> {
    run_blocking("close_open_file_holder", move || {
        OpenFileHolderService::close(path, executable_path, mode)
    })
    .await
}
