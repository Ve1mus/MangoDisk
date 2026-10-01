//! Signed entitlements of an installed application bundle.
use std::path::Path;

/// Returns the application-group identifiers declared in the bundle signature.
///
/// An unsigned bundle, an unreadable signature, or an unsupported platform yields
/// an empty list, so callers fall back to the identifiers they can verify elsewhere.
#[cfg(target_os = "macos")]
pub fn application_groups(application: &Path) -> Vec<String> {
    use std::time::Duration;

    use crate::{
        run_controlled_command, ControlledCommandLimits, ControlledEnvironmentPolicy,
        ControlledExecutable,
    };

    let Ok(executable) = ControlledExecutable::capture(Path::new("/usr/bin/codesign")) else {
        return Vec::new();
    };
    let application = application.to_string_lossy();
    // `:-` writes the entitlement plist to stdout; codesign prints its own
    // identification lines to stderr, which is not parsed.
    let output = run_controlled_command(
        "macos-application-entitlements",
        &executable,
        &["-d", "--entitlements", ":-", application.as_ref()],
        ControlledEnvironmentPolicy::Inherit,
        ControlledCommandLimits {
            timeout: Duration::from_secs(5),
            stdout_bytes: 1024 * 1024,
            stderr_bytes: 64 * 1024,
        },
        &|| false,
    );
    match output {
        Ok(output) if output.status.success() => groups_from_entitlements(&output.stdout),
        Ok(_) => Vec::new(),
        Err(error) => {
            log::debug!(
                "macos_application_entitlements_unavailable reason={}",
                error.as_str()
            );
            Vec::new()
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub fn application_groups(_application: &Path) -> Vec<String> {
    Vec::new()
}

#[cfg(any(target_os = "macos", test))]
fn groups_from_entitlements(entitlements: &[u8]) -> Vec<String> {
    let Some(dictionary) = plist::Value::from_reader(std::io::Cursor::new(entitlements))
        .ok()
        .and_then(plist::Value::into_dictionary)
    else {
        return Vec::new();
    };
    let mut groups = dictionary
        .get("com.apple.security.application-groups")
        .and_then(plist::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(plist::Value::as_string)
        .map(str::to_string)
        .collect::<Vec<_>>();
    groups.sort();
    groups.dedup();
    groups
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn groups_come_from_the_application_groups_entitlement() {
        let entitlements = br#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0"><dict>
  <key>com.apple.security.app-sandbox</key><true/>
  <key>com.apple.security.application-groups</key>
  <array>
    <string>group.com.example.Editor.shared</string>
    <string>group.com.example.Editor.shared</string>
    <string>ABCDE12345.com.example.Editor</string>
  </array>
</dict></plist>"#;

        assert_eq!(
            groups_from_entitlements(entitlements),
            vec![
                "ABCDE12345.com.example.Editor".to_string(),
                "group.com.example.Editor.shared".to_string(),
            ]
        );
    }

    #[test]
    fn missing_or_malformed_entitlements_yield_no_groups() {
        assert!(groups_from_entitlements(b"").is_empty());
        assert!(groups_from_entitlements(b"not a plist").is_empty());
        assert!(groups_from_entitlements(
            br#"<?xml version="1.0"?><plist version="1.0"><dict/></plist>"#
        )
        .is_empty());
    }
}
