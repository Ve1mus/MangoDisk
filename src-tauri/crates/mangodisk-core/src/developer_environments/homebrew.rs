//! Homebrew inventory read straight from the Cellar and Caskroom.
//!
//! `brew` is deliberately not executed: it may update itself, touch the
//! network, or take seconds. The install receipts hold everything needed to
//! tell a package the user asked for from a dependency pulled in by another.
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

use mangodisk_platform::homebrew::{self as platform_homebrew, HomebrewUninstallRun, PREFIXES};
use serde::{Deserialize, Serialize};

use super::tree_size;
use crate::{CoreError, CoreResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HomebrewPackageKind {
    Formula,
    Cask,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HomebrewPackage {
    pub name: String,
    pub kind: HomebrewPackageKind,
    pub versions: Vec<String>,
    pub bytes: u64,
    pub installed_at_ms: Option<u64>,
    /// True when the user installed the package directly rather than as a dependency.
    pub installed_on_request: bool,
    /// Installed formulae that declare this package as a runtime dependency.
    pub required_by: Vec<String>,
    /// Runtime dependencies this package declares in its install receipt.
    pub dependencies: Vec<String>,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HomebrewInventory {
    pub schema_version: u32,
    pub supported: bool,
    pub prefix: Option<String>,
    pub packages: Vec<HomebrewPackage>,
    pub total_bytes: u64,
}

pub(super) fn scan() -> HomebrewInventory {
    let Some(prefix) = installed_prefix() else {
        return HomebrewInventory {
            schema_version: super::DEVELOPER_ENVIRONMENTS_SCHEMA_VERSION,
            supported: false,
            prefix: None,
            packages: Vec::new(),
            total_bytes: 0,
        };
    };
    let mut packages = read_packages(&prefix.join("Cellar"), HomebrewPackageKind::Formula);
    packages.extend(read_packages(
        &prefix.join("Caskroom"),
        HomebrewPackageKind::Cask,
    ));
    attach_reverse_dependencies(&mut packages, &prefix.join("Cellar"));
    packages.sort_by(|left, right| {
        right
            .bytes
            .cmp(&left.bytes)
            .then_with(|| left.name.cmp(&right.name))
    });
    let total_bytes = packages
        .iter()
        .fold(0_u64, |total, package| total.saturating_add(package.bytes));
    log::info!(
        "homebrew_inventory_scanned prefix={} package_count={} total_bytes={total_bytes}",
        mangodisk_platform::diagnostics::text(&prefix.to_string_lossy()),
        packages.len()
    );
    HomebrewInventory {
        schema_version: super::DEVELOPER_ENVIRONMENTS_SCHEMA_VERSION,
        supported: true,
        prefix: Some(prefix.to_string_lossy().into_owned()),
        packages,
        total_bytes,
    }
}

fn read_packages(root: &Path, kind: HomebrewPackageKind) -> Vec<HomebrewPackage> {
    let Ok(entries) = fs::read_dir(root) else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            let path = entry.path();
            // Hidden entries are Homebrew bookkeeping, and links are never owned content.
            if name.starts_with('.') || !is_real_directory(&path) {
                return None;
            }
            package_from_directory(name, path, kind)
        })
        .collect()
}

fn package_from_directory(
    name: String,
    path: PathBuf,
    kind: HomebrewPackageKind,
) -> Option<HomebrewPackage> {
    let versions = installed_versions(&path);
    if versions.is_empty() {
        return None;
    }
    let bytes = versions.iter().fold(0_u64, |total, version| {
        total.saturating_add(tree_size::measure(&path.join(version)).bytes)
    });
    let newest = versions.last().map(|version| path.join(version))?;
    let receipt = (kind == HomebrewPackageKind::Formula)
        .then(|| read_receipt(&newest.join("INSTALL_RECEIPT.json")))
        .flatten();
    let installed_at_ms = receipt
        .as_ref()
        .and_then(|receipt| receipt.installed_at_seconds)
        .map(|seconds| seconds.saturating_mul(1_000))
        .or_else(|| modified_ms(&newest));
    Some(HomebrewPackage {
        name,
        kind,
        versions,
        bytes,
        installed_at_ms,
        // A cask is always a direct request, and a missing receipt cannot prove
        // the package was a dependency, so neither is presented as removable noise.
        installed_on_request: receipt
            .as_ref()
            .map(|receipt| receipt.installed_on_request)
            .unwrap_or(true),
        required_by: Vec::new(),
        dependencies: receipt
            .map(|receipt| receipt.runtime_dependencies)
            .unwrap_or_default(),
        path: path.to_string_lossy().into_owned(),
    })
}

#[derive(Debug, Default)]
struct Receipt {
    installed_on_request: bool,
    installed_at_seconds: Option<u64>,
    runtime_dependencies: Vec<String>,
}

fn read_receipt(path: &Path) -> Option<Receipt> {
    let bytes = fs::read(path).ok()?;
    parse_receipt(&bytes)
}

fn parse_receipt(bytes: &[u8]) -> Option<Receipt> {
    let value: serde_json::Value = serde_json::from_slice(bytes).ok()?;
    Some(Receipt {
        installed_on_request: value
            .get("installed_on_request")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false),
        installed_at_seconds: value.get("time").and_then(serde_json::Value::as_u64),
        runtime_dependencies: value
            .get("runtime_dependencies")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|dependency| dependency.get("full_name")?.as_str().map(str::to_string))
            .collect(),
    })
}

fn attach_reverse_dependencies(packages: &mut [HomebrewPackage], cellar: &Path) {
    let required_by = reverse_dependencies(cellar);
    for package in packages {
        if let Some(dependents) = required_by.get(&package.name) {
            package.required_by = dependents.iter().cloned().collect();
        }
    }
}

/// Maps every package to the installed formulae that declare it as a runtime dependency.
/// Reads install receipts only, so it stays cheap compared with measuring package sizes.
fn reverse_dependencies(cellar: &Path) -> BTreeMap<String, BTreeSet<String>> {
    let mut required_by: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let Ok(entries) = fs::read_dir(cellar) else {
        return required_by;
    };
    for entry in entries.filter_map(Result::ok) {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') || !is_real_directory(&entry.path()) {
            continue;
        }
        let Some(version) = installed_versions(&entry.path()).pop() else {
            continue;
        };
        let receipt = read_receipt(&entry.path().join(version).join("INSTALL_RECEIPT.json"));
        for dependency in receipt
            .map(|receipt| receipt.runtime_dependencies)
            .unwrap_or_default()
        {
            required_by
                .entry(dependency)
                .or_default()
                .insert(name.clone());
        }
    }
    required_by
}

/// Version directories of one package, oldest first; hidden entries and links are bookkeeping.
fn installed_versions(package: &Path) -> Vec<String> {
    let Ok(entries) = fs::read_dir(package) else {
        return Vec::new();
    };
    let mut versions = entries
        .filter_map(Result::ok)
        .filter(|entry| {
            !entry.file_name().to_string_lossy().starts_with('.')
                && is_real_directory(&entry.path())
        })
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    versions.sort();
    versions
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum HomebrewUninstallOutcome {
    Removed,
    /// The package is already gone, so there is nothing to remove.
    NotInstalled,
    /// Other installed formulae still need the package; it is left in place.
    StillRequired,
    /// `brew` reported success but the package is still installed.
    StillInstalled,
    /// `brew` failed, timed out, or could not be started.
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HomebrewUninstallResult {
    pub schema_version: u32,
    pub name: String,
    pub kind: HomebrewPackageKind,
    pub outcome: HomebrewUninstallOutcome,
    pub released_bytes: u64,
    /// Installed formulae that block the removal; empty unless the outcome is `StillRequired`.
    pub required_by: Vec<String>,
    pub exit_code: Option<i32>,
}

/// Removes one package through `brew`, which owns the links and metadata a directory delete
/// would leave behind. The package must be installed and not needed by another formula.
pub(super) fn uninstall(
    name: String,
    kind: HomebrewPackageKind,
) -> CoreResult<HomebrewUninstallResult> {
    let Some(prefix) = installed_prefix() else {
        return Err(CoreError::operation_failed("Homebrew is not installed"));
    };
    uninstall_with(&prefix, name, kind, platform_homebrew::uninstall)
}

/// The `run` seam lets tests exercise every outcome without executing a real `brew`.
fn uninstall_with(
    prefix: &Path,
    name: String,
    kind: HomebrewPackageKind,
    run: impl FnOnce(&Path, &str, bool) -> mangodisk_platform::PlatformResult<HomebrewUninstallRun>,
) -> CoreResult<HomebrewUninstallResult> {
    if !platform_homebrew::is_safe_package_name(&name) {
        return Err(CoreError::invalid_input("invalid Homebrew package name"));
    }
    let root = prefix.join(match kind {
        HomebrewPackageKind::Formula => "Cellar",
        HomebrewPackageKind::Cask => "Caskroom",
    });
    let result = |outcome, released_bytes, required_by, exit_code| HomebrewUninstallResult {
        schema_version: super::DEVELOPER_ENVIRONMENTS_SCHEMA_VERSION,
        name: name.clone(),
        kind,
        outcome,
        released_bytes,
        required_by,
        exit_code,
    };
    // A linked package directory is never owned content, as in the inventory scan.
    let package_dir = root.join(&name);
    let Some(package) = is_real_directory(&package_dir)
        .then(|| package_from_directory(name.clone(), package_dir, kind))
        .flatten()
    else {
        return Ok(result(
            HomebrewUninstallOutcome::NotInstalled,
            0,
            Vec::new(),
            None,
        ));
    };
    // Checked here as well as by `brew`, so the answer does not depend on its wording.
    let blockers = (kind == HomebrewPackageKind::Formula)
        .then(|| reverse_dependencies(&prefix.join("Cellar")))
        .and_then(|mut required_by| required_by.remove(&name))
        .map(|dependents| dependents.into_iter().collect::<Vec<_>>())
        .unwrap_or_default();
    if !blockers.is_empty() {
        log::warn!(
            "homebrew_uninstall_blocked name={} dependent_count={}",
            mangodisk_platform::diagnostics::text(&name),
            blockers.len()
        );
        return Ok(result(
            HomebrewUninstallOutcome::StillRequired,
            0,
            blockers,
            None,
        ));
    }
    let started = std::time::Instant::now();
    let run = run(prefix, &name, kind == HomebrewPackageKind::Cask);
    // Verify on disk rather than trusting the exit code: a timeout can still have removed it.
    let still_installed = package_from_directory(name.clone(), root.join(&name), kind).is_some();
    let (outcome, exit_code) = match (&run, still_installed) {
        (_, false) => (HomebrewUninstallOutcome::Removed, None),
        (Ok(HomebrewUninstallRun::Succeeded), true) => {
            (HomebrewUninstallOutcome::StillInstalled, Some(0))
        }
        (Ok(HomebrewUninstallRun::Failed { exit_code }), true) => {
            (HomebrewUninstallOutcome::Failed, *exit_code)
        }
        (Err(_), true) => (HomebrewUninstallOutcome::Failed, None),
    };
    log::info!(
        "homebrew_uninstall_finished name={} kind={kind:?} outcome={outcome:?} exit_code={exit_code:?} error={} elapsed_ms={}",
        mangodisk_platform::diagnostics::text(&name),
        run.as_ref()
            .err()
            .map(mangodisk_platform::diagnostics::text)
            .unwrap_or_default(),
        started.elapsed().as_millis()
    );
    let released_bytes = if outcome == HomebrewUninstallOutcome::Removed {
        package.bytes
    } else {
        0
    };
    Ok(result(outcome, released_bytes, Vec::new(), exit_code))
}

fn installed_prefix() -> Option<PathBuf> {
    PREFIXES
        .iter()
        .map(PathBuf::from)
        .find(|prefix| prefix.join("Cellar").is_dir() || prefix.join("Caskroom").is_dir())
}

fn is_real_directory(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|metadata| metadata.is_dir())
}

fn modified_ms(path: &Path) -> Option<u64> {
    let modified = fs::metadata(path).ok()?.modified().ok()?;
    u64::try_from(modified.duration_since(UNIX_EPOCH).ok()?.as_millis()).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn formula(root: &Path, name: &str, version: &str, receipt: &str) {
        let directory = root.join("Cellar").join(name).join(version);
        fs::create_dir_all(directory.join("bin")).unwrap();
        fs::write(directory.join("bin/tool"), [0_u8; 100]).unwrap();
        fs::write(directory.join("INSTALL_RECEIPT.json"), receipt).unwrap();
    }

    #[test]
    fn receipt_separates_requested_packages_from_dependencies() {
        let receipt = parse_receipt(
            br#"{"installed_on_request": false, "time": 1760497979,
                 "runtime_dependencies": [{"full_name": "dbus"}, {"full_name": "glib"}]}"#,
        )
        .unwrap();

        assert!(!receipt.installed_on_request);
        assert_eq!(receipt.installed_at_seconds, Some(1_760_497_979));
        assert_eq!(receipt.runtime_dependencies, ["dbus", "glib"]);
        assert!(parse_receipt(b"not json").is_none());
    }

    #[test]
    fn reverse_dependencies_are_attached_to_installed_formulae() {
        let root = tempfile::tempdir().unwrap();
        formula(
            root.path(),
            "app",
            "1.0",
            r#"{"installed_on_request": true, "runtime_dependencies": [{"full_name": "lib"}]}"#,
        );
        formula(
            root.path(),
            "lib",
            "2.1",
            r#"{"installed_on_request": false}"#,
        );
        let mut packages = read_packages(&root.path().join("Cellar"), HomebrewPackageKind::Formula);
        attach_reverse_dependencies(&mut packages, &root.path().join("Cellar"));
        let lib = packages
            .iter()
            .find(|package| package.name == "lib")
            .unwrap();
        let app = packages
            .iter()
            .find(|package| package.name == "app")
            .unwrap();

        assert_eq!(lib.required_by, ["app"]);
        assert!(!lib.installed_on_request);
        assert!(app.installed_on_request);
        assert!(app.required_by.is_empty());
        assert_eq!(lib.versions, ["2.1"]);
        assert!(lib.bytes >= 100);
    }

    #[test]
    fn packages_list_their_own_runtime_dependencies() {
        let root = tempfile::tempdir().unwrap();
        formula(
            root.path(),
            "app",
            "1.0",
            r#"{"installed_on_request": true, "runtime_dependencies": [{"full_name": "lib"}, {"full_name": "ssl"}]}"#,
        );

        let packages = read_packages(&root.path().join("Cellar"), HomebrewPackageKind::Formula);

        assert_eq!(packages[0].dependencies, ["lib", "ssl"]);
    }

    fn two_packages(root: &Path) {
        formula(
            root,
            "app",
            "1.0",
            r#"{"installed_on_request": true, "runtime_dependencies": [{"full_name": "lib"}]}"#,
        );
        formula(root, "lib", "2.1", r#"{"installed_on_request": false}"#);
    }

    fn remove_package(
        root: &Path,
        name: &'static str,
    ) -> impl FnOnce(&Path, &str, bool) -> mangodisk_platform::PlatformResult<HomebrewUninstallRun>
    {
        let target = root.join("Cellar").join(name);
        move |_, _, _| {
            fs::remove_dir_all(target).unwrap();
            Ok(HomebrewUninstallRun::Succeeded)
        }
    }

    #[test]
    fn a_package_other_formulae_need_is_never_handed_to_brew() {
        let root = tempfile::tempdir().unwrap();
        two_packages(root.path());

        let result = uninstall_with(
            root.path(),
            "lib".to_string(),
            HomebrewPackageKind::Formula,
            |_, _, _| panic!("a required package must not reach brew"),
        )
        .unwrap();

        assert_eq!(result.outcome, HomebrewUninstallOutcome::StillRequired);
        assert_eq!(result.required_by, ["app"]);
        assert!(root.path().join("Cellar/lib").exists());
    }

    #[test]
    fn a_removed_package_is_verified_on_disk_and_its_size_reported() {
        let root = tempfile::tempdir().unwrap();
        two_packages(root.path());

        let result = uninstall_with(
            root.path(),
            "app".to_string(),
            HomebrewPackageKind::Formula,
            remove_package(root.path(), "app"),
        )
        .unwrap();

        assert_eq!(result.outcome, HomebrewUninstallOutcome::Removed);
        assert!(result.released_bytes >= 100);
    }

    #[test]
    fn failures_and_false_success_are_reported_from_the_disk_state() {
        let root = tempfile::tempdir().unwrap();
        two_packages(root.path());
        let outcome = |run: HomebrewUninstallRun| {
            uninstall_with(
                root.path(),
                "app".to_string(),
                HomebrewPackageKind::Formula,
                move |_, _, _| Ok(run),
            )
            .unwrap()
        };

        let failed = outcome(HomebrewUninstallRun::Failed { exit_code: Some(1) });
        assert_eq!(failed.outcome, HomebrewUninstallOutcome::Failed);
        assert_eq!(failed.exit_code, Some(1));
        let phantom = outcome(HomebrewUninstallRun::Succeeded);
        assert_eq!(phantom.outcome, HomebrewUninstallOutcome::StillInstalled);
        let missing = uninstall_with(
            root.path(),
            "ghost".to_string(),
            HomebrewPackageKind::Formula,
            |_, _, _| panic!("nothing to run"),
        )
        .unwrap();
        assert_eq!(missing.outcome, HomebrewUninstallOutcome::NotInstalled);
    }

    #[test]
    fn unsafe_names_are_rejected_before_anything_runs() {
        let root = tempfile::tempdir().unwrap();

        for name in ["--force", "tap/x", "../Cellar"] {
            let error = uninstall_with(
                root.path(),
                name.to_string(),
                HomebrewPackageKind::Formula,
                |_, _, _| panic!("must not run"),
            );
            assert!(error.is_err(), "{name}");
        }
    }

    #[test]
    fn hidden_entries_and_empty_packages_are_ignored() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir_all(root.path().join("Caskroom/.metadata")).unwrap();
        fs::create_dir_all(root.path().join("Caskroom/empty")).unwrap();
        fs::create_dir_all(root.path().join("Caskroom/editor/1.0")).unwrap();

        let packages = read_packages(&root.path().join("Caskroom"), HomebrewPackageKind::Cask);

        assert_eq!(packages.len(), 1);
        assert_eq!(packages[0].name, "editor");
        assert!(packages[0].installed_on_request);
    }
}
