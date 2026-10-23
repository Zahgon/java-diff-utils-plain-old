//! Applying a multi-file diff to a tree of files.

use std::collections::BTreeMap;

use crate::unifieddiff::{UnifiedDiff, UnifiedDiffFile};

/// Why a multi-file diff did not apply: the index of the file in `diff.files`, the index of the rejected delta
/// when the patch of that file failed on one, and the reason.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreeApplyFailed {
    pub file: usize,
    pub delta: Option<usize>,
    pub reason: String,
}

/// Apply every file of `diff` to `tree` (file path to lines), in order, all or nothing: on failure `tree` is
/// left exactly as it was.
pub fn apply_unified_diff(diff: &UnifiedDiff, tree: &mut BTreeMap<String, Vec<String>>) -> Result<(), TreeApplyFailed> {
    let mut work = tree.clone();
    let result = apply_files(diff, &mut work);
    if result.is_ok() {
        *tree = work;
    }
    result
}

fn apply_files(diff: &UnifiedDiff, tree: &mut BTreeMap<String, Vec<String>>) -> Result<(), TreeApplyFailed> {
    for (index, file) in diff.files.iter().enumerate() {
        apply_file(file, tree).map_err(|(delta, reason)| TreeApplyFailed { file: index, delta, reason })?;
    }
    Ok(())
}

fn path(name: Option<&String>) -> Option<&str> {
    name.map(String::as_str).filter(|name| *name != "/dev/null")
}

fn apply_file(file: &UnifiedDiffFile, tree: &mut BTreeMap<String, Vec<String>>) -> Result<(), (Option<usize>, String)> {
    let fail = |reason: &str| (None, reason.to_string());
    if file.binary_added.is_some() || file.binary_deleted.is_some() || file.binary_edited.is_some() {
        return Err(fail("BINARY"));
    }
    let old = path(file.rename_from.as_ref().or(file.copy_from.as_ref()).or(file.from_file.as_ref()));
    let new = path(file.rename_to.as_ref().or(file.copy_to.as_ref()).or(file.to_file.as_ref()));
    let mut lines = match old {
        None if new.is_none() => return Err(fail("NO_PATH")),
        None => Vec::new(),
        Some(old) => tree.get(old).cloned().ok_or_else(|| fail("MISSING_FILE"))?,
    };
    if let Some(new) = new
        && old != Some(new)
        && tree.contains_key(new)
    {
        return Err(fail("FILE_EXISTS"));
    }
    file.patch.apply_to(&mut lines).map_err(|failure| (failure.delta, failure.reason))?;
    match (old, new) {
        (Some(old), None) => {
            if !lines.is_empty() {
                return Err(fail("NOT_EMPTY"));
            }
            tree.remove(old);
        }
        (old, Some(new)) => {
            if let Some(old) = old.filter(|old| *old != new && file.copy_from.is_none()) {
                tree.remove(old);
            }
            tree.insert(new.to_string(), lines);
        }
        (None, None) => {}
    }
    Ok(())
}
