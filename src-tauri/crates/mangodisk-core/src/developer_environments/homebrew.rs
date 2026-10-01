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

use serde::Serialize;

use super::tree_size;

const PREFIXES: [&str; 3] = ["/opt/homebrew", "/usr/local", "/home/linuxbrew/.linuxbrew"];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
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
    let Some(prefix) = PREFIXES
        .iter()
        .map(Path::new)
        .find(|prefix| prefix.join("Cellar").is_dir() || prefix.join("Caskroom").is_dir())
    else {
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
    let mut versions = fs::read_dir(&path)
        .ok()?
        .filter_map(Result::ok)
        .filter(|entry| {
            !entry.file_name().to_string_lossy().starts_with('.')
                && is_real_directory(&entry.path())
        })
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    if versions.is_empty() {
        return None;
    }
    versions.sort();
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
    let mut required_by: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for package in packages
        .iter()
        .filter(|package| package.kind == HomebrewPackageKind::Formula)
    {
        let Some(version) = package.versions.last() else {
            continue;
        };
        let receipt = read_receipt(
            &cellar
                .join(&package.name)
                .join(version)
                .join("INSTALL_RECEIPT.json"),
        );
        for dependency in receipt
            .map(|receipt| receipt.runtime_dependencies)
            .unwrap_or_default()
        {
            required_by
                .entry(dependency)
                .or_default()
                .insert(package.name.clone());
        }
    }
    for package in packages {
        if let Some(dependents) = required_by.get(&package.name) {
            package.required_by = dependents.iter().cloned().collect();
        }
    }
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
