use mangodisk_core::{DeveloperEnvironmentService, HomebrewInventory, PythonEnvironmentScan};

use super::error::{run_blocking, CommandResult};

#[tauri::command]
pub async fn scan_homebrew_packages() -> CommandResult<HomebrewInventory> {
    run_blocking(
        "scan_homebrew_packages",
        DeveloperEnvironmentService::scan_homebrew,
    )
    .await
}

#[tauri::command]
pub async fn scan_python_environments() -> CommandResult<PythonEnvironmentScan> {
    run_blocking(
        "scan_python_environments",
        DeveloperEnvironmentService::scan_python_environments,
    )
    .await
}
