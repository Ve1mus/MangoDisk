//! Homebrew locations and the one command MangoDisk runs through `brew`.
//!
//! Package inventory is read from the Cellar without executing anything; removing
//! a package is left to `brew` itself because it also owns the symlinks, services,
//! and cask artifacts that a plain directory delete would leave behind.
use std::path::Path;

use crate::{PlatformError, PlatformErrorCode, PlatformResult};

/// Install prefixes Homebrew uses on Apple Silicon, Intel macOS, and Linux.
pub const PREFIXES: [&str; 3] = ["/opt/homebrew", "/usr/local", "/home/linuxbrew/.linuxbrew"];

/// Largest package name Homebrew accepts in practice; guards against absurd input.
const MAX_PACKAGE_NAME_LENGTH: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HomebrewUninstallRun {
    Succeeded,
    Failed { exit_code: Option<i32> },
}

/// A plain package name: no tap prefix, path separator, or leading dash, so the
/// value can only ever be read by `brew` as a package and never as an option.
pub fn is_safe_package_name(name: &str) -> bool {
    let mut characters = name.chars();
    name.len() <= MAX_PACKAGE_NAME_LENGTH
        && characters
            .next()
            .is_some_and(|first| first.is_ascii_alphanumeric())
        && characters.all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '@' | '+' | '.' | '_' | '-')
        })
}

/// Runs `brew uninstall` for one installed formula or cask.
///
/// Deliberately passes neither `--force` nor `--ignore-dependencies`, so Homebrew
/// keeps its own refusal to remove a package other packages still need.
#[cfg(unix)]
pub fn uninstall(prefix: &Path, name: &str, cask: bool) -> PlatformResult<HomebrewUninstallRun> {
    use std::time::Duration;

    use crate::{
        run_controlled_command, ControlledCommandError, ControlledCommandLimits,
        ControlledEnvironmentPolicy, ControlledExecutable, PlatformFailureReason,
    };

    if !PREFIXES.iter().any(|known| Path::new(known) == prefix) {
        return Err(PlatformError::invalid_path(
            "homebrew_prefix_not_recognized".to_string(),
        ));
    }
    if !is_safe_package_name(name) {
        return Err(PlatformError::new(
            PlatformErrorCode::InvalidData,
            "homebrew_package_name_invalid",
        ));
    }
    let executable = ControlledExecutable::capture(&prefix.join("bin/brew")).map_err(|error| {
        PlatformError::operation_failed(format!("homebrew_tool_invalid reason={}", error.as_str()))
            .with_failure_reason(PlatformFailureReason::ToolUnavailable)
    })?;
    let kind = if cask { "--cask" } else { "--formula" };
    let output = run_controlled_command(
        "homebrew-uninstall",
        &executable,
        &["uninstall", kind, name],
        // brew needs the desktop environment (HOME, locale, proxy settings) to run.
        ControlledEnvironmentPolicy::Inherit,
        ControlledCommandLimits {
            timeout: Duration::from_secs(300),
            stdout_bytes: 256 * 1024,
            stderr_bytes: 256 * 1024,
        },
        &|| false,
    )
    .map_err(|error| {
        let reason = match error {
            ControlledCommandError::TimedOut => PlatformFailureReason::TimedOut,
            _ => PlatformFailureReason::ServiceUnavailable,
        };
        PlatformError::operation_failed(format!(
            "homebrew_uninstall_command_failed reason={}",
            error.as_str()
        ))
        .with_failure_reason(reason)
        // The child may have removed files before it was stopped.
        .with_possible_side_effects()
    })?;
    Ok(if output.status.success() {
        HomebrewUninstallRun::Succeeded
    } else {
        HomebrewUninstallRun::Failed {
            exit_code: output.status.code(),
        }
    })
}

#[cfg(not(unix))]
pub fn uninstall(_prefix: &Path, _name: &str, _cask: bool) -> PlatformResult<HomebrewUninstallRun> {
    Err(PlatformError::new(
        PlatformErrorCode::Unsupported,
        "homebrew is not supported on this platform",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_names_cannot_be_read_as_options_paths_or_taps() {
        for valid in [
            "wget",
            "python@3.13",
            "gcc",
            "c++",
            "node_exporter",
            "ffmpeg-full",
        ] {
            assert!(is_safe_package_name(valid), "{valid}");
        }
        for invalid in [
            "",
            "-rf",
            "--force",
            ".hidden",
            "tap/formula",
            "../x",
            "a b",
            "a;b",
            "bad\nname",
            &"x".repeat(129),
        ] {
            assert!(!is_safe_package_name(invalid), "{invalid:?}");
        }
    }

    #[test]
    #[cfg(unix)]
    fn refuses_an_unrecognized_prefix_before_running_anything() {
        let error = uninstall(Path::new("/tmp/fake-brew"), "wget", false)
            .expect_err("only Homebrew's own prefixes are trusted");

        assert!(error.to_string().contains("homebrew_prefix_not_recognized"));
    }
}
