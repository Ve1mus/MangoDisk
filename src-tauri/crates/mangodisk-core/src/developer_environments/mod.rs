//! Read-only inventory of developer-installed software: Homebrew packages and
//! Python virtual environments. Nothing here deletes or modifies anything; the
//! results help a user decide what to remove with the tool that owns it.
mod homebrew;
mod python_environments;
mod tree_size;

pub use homebrew::{HomebrewInventory, HomebrewPackage, HomebrewPackageKind};
pub use python_environments::{PythonEnvironment, PythonEnvironmentScan};

use crate::CoreResult;

pub const DEVELOPER_ENVIRONMENTS_SCHEMA_VERSION: u32 = 1;

pub struct DeveloperEnvironmentService;

impl DeveloperEnvironmentService {
    pub fn scan_homebrew() -> CoreResult<HomebrewInventory> {
        Ok(homebrew::scan())
    }

    pub fn scan_python_environments() -> CoreResult<PythonEnvironmentScan> {
        python_environments::scan()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "scans the real Homebrew prefix and home directory"]
    fn scans_the_installed_machine() {
        let homebrew = DeveloperEnvironmentService::scan_homebrew().unwrap();
        println!(
            "homebrew supported={} packages={} bytes={}",
            homebrew.supported,
            homebrew.packages.len(),
            homebrew.total_bytes
        );
        for package in homebrew.packages.iter().take(5) {
            println!(
                "  {} {:?} {:?} onRequest={} requiredBy={}",
                package.name,
                package.kind,
                package.versions,
                package.installed_on_request,
                package.required_by.len()
            );
        }
        let python = DeveloperEnvironmentService::scan_python_environments().unwrap();
        println!(
            "python environments={} bytes={} complete={} elapsed_ms={}",
            python.environments.len(),
            python.total_bytes,
            python.complete,
            python.elapsed_ms
        );
        for item in &python.environments {
            println!(
                "  {} v={:?} bytes={} markers={} homeRoot={} tool={} missing={}",
                item.path,
                item.python_version,
                item.bytes,
                item.has_project_markers,
                item.in_home_root,
                item.tool_managed,
                item.interpreter_missing
            );
        }
    }
}
