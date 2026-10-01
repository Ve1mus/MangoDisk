//! Discovery of Python virtual environments under the user's home directory.
//!
//! A directory is an environment when it holds a `pyvenv.cfg`, the marker every
//! `venv`, `virtualenv`, and `uv` environment writes. The scan never follows
//! symbolic links and does not descend into an environment once found.
use std::{
    fs,
    path::{Path, PathBuf},
    time::{Instant, UNIX_EPOCH},
};

use mangodisk_platform::{current_platform, Platform};
use serde::Serialize;

use super::tree_size;
use crate::{CoreError, CoreResult};

const MAX_DEPTH: usize = 8;
const MAX_DIRECTORIES: u64 = 400_000;
const SCAN_TIME_LIMIT_MS: u128 = 30_000;
/// Directories that cannot contain a user's environments or are costly and noisy to walk.
const SKIPPED_DIRECTORIES: [&str; 12] = [
    "Library",
    ".Trash",
    "node_modules",
    ".git",
    "Applications",
    "Movies",
    "Music",
    "Pictures",
    ".npm",
    ".cargo",
    ".rustup",
    "target",
];
const PROJECT_MARKERS: [&str; 9] = [
    "pyproject.toml",
    "requirements.txt",
    "setup.py",
    "setup.cfg",
    "Pipfile",
    "poetry.lock",
    "uv.lock",
    "environment.yml",
    ".git",
];
/// Locations owned by tools that create and recycle environments on their own.
const TOOL_MANAGED_FRAGMENTS: [&str; 5] = [
    ".cache/uv",
    ".local/share/uv",
    ".local/pipx",
    ".platformio",
    ".pyenv/versions",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PythonEnvironment {
    pub path: String,
    pub python_version: Option<String>,
    /// The interpreter directory named by `pyvenv.cfg` no longer exists.
    pub interpreter_missing: bool,
    pub include_system_site_packages: bool,
    /// A project file (`pyproject.toml`, `requirements.txt`, …) sits beside the environment.
    pub has_project_markers: bool,
    /// The environment sits directly in the home directory, a common accidental global install.
    pub in_home_root: bool,
    /// Owned by a tool such as uv or PlatformIO, which recreates it on demand.
    pub tool_managed: bool,
    pub bytes: u64,
    pub file_count: u64,
    pub created_at_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PythonEnvironmentScan {
    pub schema_version: u32,
    pub environments: Vec<PythonEnvironment>,
    pub total_bytes: u64,
    /// False when a depth, entry, or time limit stopped the search early.
    pub complete: bool,
    pub elapsed_ms: u64,
}

pub(super) fn scan() -> CoreResult<PythonEnvironmentScan> {
    let user_directories = current_platform()
        .user_directories()
        .map_err(|error| CoreError::operation_failed(error.to_string()))?;
    let home = user_directories.home_directory().to_path_buf();
    Ok(scan_root(&home, &home))
}

fn scan_root(root: &Path, home: &Path) -> PythonEnvironmentScan {
    let started = Instant::now();
    let mut complete = true;
    let mut visited = 0_u64;
    let mut found = Vec::new();
    let mut pending = vec![(root.to_path_buf(), 0_usize)];
    while let Some((directory, depth)) = pending.pop() {
        if visited >= MAX_DIRECTORIES || started.elapsed().as_millis() > SCAN_TIME_LIMIT_MS {
            complete = false;
            break;
        }
        visited += 1;
        if directory.join("pyvenv.cfg").is_file() {
            found.push(directory);
            continue;
        }
        // Environments nested deeper than the limit are not expected and are not an
        // incomplete result; only an exhausted entry or time budget is.
        if depth >= MAX_DEPTH {
            continue;
        }
        let Ok(entries) = fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.filter_map(Result::ok) {
            let name = entry.file_name();
            if SKIPPED_DIRECTORIES
                .iter()
                .any(|skipped| name == std::ffi::OsStr::new(skipped))
            {
                continue;
            }
            // `symlink_metadata` keeps links out of the walk, so a loop or an
            // outside tree can never be entered through one.
            if entry
                .path()
                .symlink_metadata()
                .is_ok_and(|metadata| metadata.is_dir())
            {
                pending.push((entry.path(), depth + 1));
            }
        }
    }

    let mut environments = found
        .into_iter()
        .map(|path| describe(&path, home))
        .collect::<Vec<_>>();
    environments.sort_by(|left, right| {
        right
            .bytes
            .cmp(&left.bytes)
            .then_with(|| left.path.cmp(&right.path))
    });
    let total_bytes = environments
        .iter()
        .fold(0_u64, |total, item| total.saturating_add(item.bytes));
    let elapsed_ms = started.elapsed().as_millis() as u64;
    log::info!(
        "python_environments_scanned count={} total_bytes={total_bytes} directories={visited} complete={complete} elapsed_ms={elapsed_ms}",
        environments.len()
    );
    PythonEnvironmentScan {
        schema_version: super::DEVELOPER_ENVIRONMENTS_SCHEMA_VERSION,
        environments,
        total_bytes,
        complete,
        elapsed_ms,
    }
}

fn describe(path: &Path, home: &Path) -> PythonEnvironment {
    let config_path = path.join("pyvenv.cfg");
    let config = fs::read_to_string(&config_path)
        .map(|text| parse_config(&text))
        .unwrap_or_default();
    let parent = path.parent();
    let size = tree_size::measure(path);
    PythonEnvironment {
        path: path.to_string_lossy().into_owned(),
        python_version: config.version,
        interpreter_missing: config
            .home
            .as_deref()
            .is_some_and(|interpreter_home| !Path::new(interpreter_home).exists()),
        include_system_site_packages: config.include_system_site_packages,
        has_project_markers: parent.is_some_and(has_project_marker),
        in_home_root: parent == Some(home),
        tool_managed: is_tool_managed(path, home),
        bytes: size.bytes,
        file_count: size.file_count,
        created_at_ms: fs::metadata(&config_path)
            .ok()
            .and_then(|metadata| metadata.modified().ok())
            .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
            .and_then(|duration| u64::try_from(duration.as_millis()).ok()),
    }
}

fn has_project_marker(directory: &Path) -> bool {
    PROJECT_MARKERS
        .iter()
        .any(|marker| directory.join(marker).exists())
}

fn is_tool_managed(path: &Path, home: &Path) -> bool {
    let relative = path.strip_prefix(home).unwrap_or(path);
    let relative = PathBuf::from(relative).to_string_lossy().replace('\\', "/");
    TOOL_MANAGED_FRAGMENTS
        .iter()
        .any(|fragment| relative.starts_with(fragment))
}

#[derive(Debug, Default, PartialEq, Eq)]
struct EnvironmentConfig {
    home: Option<String>,
    version: Option<String>,
    include_system_site_packages: bool,
}

/// `pyvenv.cfg` is a flat `key = value` file; `version` is `version_info` for virtualenv.
fn parse_config(text: &str) -> EnvironmentConfig {
    let mut config = EnvironmentConfig::default();
    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let (key, value) = (key.trim(), value.trim());
        match key {
            "home" => config.home = Some(value.to_string()),
            "version" | "version_info" if config.version.is_none() => {
                config.version = Some(value.to_string());
            }
            "include-system-site-packages" => {
                config.include_system_site_packages = value.eq_ignore_ascii_case("true");
            }
            _ => {}
        }
    }
    config
}

#[cfg(test)]
mod tests {
    use super::*;

    fn environment(path: &Path, config: &str) {
        fs::create_dir_all(path.join("lib")).unwrap();
        fs::write(path.join("pyvenv.cfg"), config).unwrap();
        fs::write(path.join("lib/module.py"), [0_u8; 64]).unwrap();
    }

    #[test]
    fn config_exposes_interpreter_version_and_site_packages_mode() {
        let config = parse_config(
            "home = /usr/bin\ninclude-system-site-packages = true\nversion = 3.12.4\nprompt = x\n",
        );

        assert_eq!(config.home.as_deref(), Some("/usr/bin"));
        assert_eq!(config.version.as_deref(), Some("3.12.4"));
        assert!(config.include_system_site_packages);
        assert_eq!(
            parse_config("version_info = 3.11.2.final.0")
                .version
                .as_deref(),
            Some("3.11.2.final.0")
        );
        assert_eq!(parse_config("garbage"), EnvironmentConfig::default());
    }

    #[test]
    fn finds_environments_and_flags_accidental_global_ones() {
        let home = tempfile::tempdir().unwrap();
        let project = home.path().join("work/api");
        fs::create_dir_all(&project).unwrap();
        fs::write(project.join("pyproject.toml"), "").unwrap();
        environment(
            &project.join(".venv"),
            "home = /nonexistent-interpreter\nversion = 3.12.1\n",
        );
        environment(&home.path().join(".venv"), "version = 3.13.0\n");
        environment(&home.path().join("scratch/venv"), "version = 3.10.0\n");
        // Environments must not be searched inside skipped directories.
        environment(
            &home.path().join("node_modules/pkg/.venv"),
            "version = 3.9.0\n",
        );

        let scan = scan_root(home.path(), home.path());
        let find = |suffix: &str| {
            scan.environments
                .iter()
                .find(|item| Path::new(&item.path).ends_with(suffix))
                .unwrap_or_else(|| panic!("missing {suffix}"))
        };

        assert_eq!(scan.environments.len(), 3);
        assert!(scan.complete);
        let project_env = find("work/api/.venv");
        assert!(project_env.has_project_markers && !project_env.in_home_root);
        assert!(project_env.interpreter_missing);
        let global_env = scan
            .environments
            .iter()
            .find(|item| item.in_home_root)
            .expect("the home-root environment must be flagged");
        assert!(!global_env.has_project_markers);
        let scratch = find("scratch/venv");
        assert!(!scratch.has_project_markers && !scratch.in_home_root);
        assert!(scratch.bytes >= 64 && scratch.file_count >= 2);
    }

    #[test]
    fn tool_managed_locations_are_recognised_relative_to_home() {
        let home = Path::new("/Users/example");

        assert!(is_tool_managed(
            Path::new("/Users/example/.cache/uv/archive-v0/abc"),
            home
        ));
        assert!(is_tool_managed(
            Path::new("/Users/example/.local/share/uv/tools/graphify"),
            home
        ));
        assert!(!is_tool_managed(
            Path::new("/Users/example/projects/app/.venv"),
            home
        ));
    }

    #[test]
    #[cfg(unix)]
    fn symlinked_directories_are_not_entered() {
        let home = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        environment(&outside.path().join(".venv"), "version = 3.12.0\n");
        std::os::unix::fs::symlink(outside.path(), home.path().join("linked")).unwrap();

        let scan = scan_root(home.path(), home.path());

        assert!(scan.environments.is_empty());
    }
}
