use mangodisk_core::{
    DeveloperEnvironmentService, HomebrewInventory, HomebrewPackageKind, HomebrewUninstallResult,
    PythonEnvironmentDeleteResult, PythonEnvironmentScan,
};

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

#[tauri::command]
pub async fn uninstall_homebrew_package(
    name: String,
    kind: HomebrewPackageKind,
) -> CommandResult<HomebrewUninstallResult> {
    run_blocking("uninstall_homebrew_package", move || {
        DeveloperEnvironmentService::uninstall_homebrew_package(name, kind)
    })
    .await
}

#[tauri::command]
pub async fn delete_python_environment(
    path: String,
) -> CommandResult<PythonEnvironmentDeleteResult> {
    run_blocking("delete_python_environment", move || {
        DeveloperEnvironmentService::delete_python_environment(path)
    })
    .await
}
