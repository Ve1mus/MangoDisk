//! Bounded size measurement that never follows symbolic links.
use std::{fs, path::Path};

/// Upper bound on directory entries visited by one measurement.
const MAX_ENTRIES: u64 = 2_000_000;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(super) struct TreeSize {
    pub(super) bytes: u64,
    pub(super) file_count: u64,
    /// False when the entry budget was exhausted or an entry could not be read.
    pub(super) complete: bool,
}

pub(super) fn measure(root: &Path) -> TreeSize {
    let mut size = TreeSize {
        complete: true,
        ..TreeSize::default()
    };
    let mut visited = 0_u64;
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        let Ok(entries) = fs::read_dir(&directory) else {
            size.complete = false;
            continue;
        };
        for entry in entries {
            visited += 1;
            if visited > MAX_ENTRIES {
                size.complete = false;
                return size;
            }
            let Ok(entry) = entry else {
                size.complete = false;
                continue;
            };
            let Ok(metadata) = fs::symlink_metadata(entry.path()) else {
                size.complete = false;
                continue;
            };
            if metadata.is_dir() {
                pending.push(entry.path());
            } else {
                size.bytes = size.bytes.saturating_add(metadata.len());
                size.file_count += 1;
            }
        }
    }
    size
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sums_regular_files_and_ignores_symlink_targets() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("nested")).unwrap();
        fs::write(root.path().join("a"), [0_u8; 10]).unwrap();
        fs::write(root.path().join("nested/b"), [0_u8; 5]).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(root.path().join("a"), root.path().join("link")).unwrap();

        let size = measure(root.path());

        assert_eq!(size.file_count, 2 + usize::from(cfg!(unix)) as u64);
        assert!(size.complete);
        assert!(size.bytes >= 15);
    }
}
