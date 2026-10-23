//! The three-way merge, the conflict output hook and applying a multi-file diff to a tree.

mod common;

use std::collections::BTreeMap;

use common::lines;
use diffutils::{
    ConflictOutput, Delta, PatchFailed, VerifyChunk, apply_unified_diff, diff, merge3, parse_unified_diff, read_unified_diff,
};

#[test]
fn touching_edits_conflict() {
    let base = lines(&["a", "b", "c", "d"]);
    let merged = merge3(&base, &lines(&["a", "B", "c", "d"]), &lines(&["a", "b", "C", "d"]));
    assert_eq!(merged.conflicts, 1);
    assert_eq!(
        merged.lines,
        lines(&["a", "<<<<<<< ours", "B", "c", "||||||| base", "b", "c", "=======", "b", "C", ">>>>>>> theirs", "d"])
    );
}

#[test]
fn separate_and_identical_edits_merge() {
    let base = lines(&["a", "b", "c", "d", "e"]);
    let merged = merge3(&base, &lines(&["A", "b", "c", "d", "E"]), &lines(&["A", "b", "c", "d", "e", "f"]));
    assert_eq!(merged.conflicts, 1);
    let merged = merge3(&base, &lines(&["A", "b", "c", "d", "e"]), &lines(&["a", "b", "c", "D", "e"]));
    assert_eq!((merged.conflicts, merged.lines), (0, lines(&["A", "b", "c", "D", "e"])));
    let merged = merge3(&base, &lines(&["a", "x", "c", "d", "e"]), &lines(&["a", "x", "c", "d", "e"]));
    assert_eq!((merged.conflicts, merged.lines), (0, lines(&["a", "x", "c", "d", "e"])));
}

#[test]
fn generic_elements_diff_and_apply() {
    let original = vec![1i64, 2, 3, 4];
    let revised = vec![1i64, 5, 3, 4, 6];
    let patch = diff(&original, &revised);
    let mut work = original.clone();
    patch.apply_to(&mut work).expect("the patch fits");
    assert_eq!(work, revised);
}

struct Marker;

impl ConflictOutput<String> for Marker {
    fn process_conflict(&self, verify: VerifyChunk, delta: &Delta, result: &mut Vec<String>) -> Result<(), PatchFailed> {
        let at = delta.source.position.min(result.len());
        result.insert(at, format!("!! {verify:?} {}", delta.source.position));
        Ok(())
    }
}

#[test]
fn conflict_output_sees_the_partial_result() {
    let patch = parse_unified_diff(&lines(&["--- a", "+++ b", "@@ -1 +1 @@", "-a", "+A", "@@ -3 +3 @@", "-c", "+C"]));
    let mut target = lines(&["a", "b", "x"]);
    patch.apply_to_with_conflict_output(&mut target, &Marker).expect("the marker never fails");
    assert_eq!(target, lines(&["A", "b", "!! ContentDoesNotMatchTarget 2", "x"]));
}

fn tree(files: &[(&str, &[&str])]) -> BTreeMap<String, Vec<String>> {
    files.iter().map(|(name, content)| (name.to_string(), lines(content))).collect()
}

#[test]
fn tree_is_unchanged_when_a_later_file_fails() {
    let text = "diff --git a/x b/x\n--- a/x\n+++ b/x\n@@ -1 +1 @@\n-a\n+b\ndiff --git a/y b/y\n--- a/y\n+++ b/y\n@@ -1 +1 @@\n-q\n+r\n";
    let diff = read_unified_diff(text).expect("a well-formed diff");
    let mut files = tree(&[("x", &["a"]), ("y", &["p"])]);
    let failure = apply_unified_diff(&diff, &mut files).expect_err("y does not match");
    assert_eq!((failure.file, failure.delta, failure.reason.as_str()), (1, Some(0), "CONTENT_DOES_NOT_MATCH_TARGET"));
    assert_eq!(files, tree(&[("x", &["a"]), ("y", &["p"])]));
}

#[test]
fn tree_renames_and_refuses_to_overwrite() {
    let text = "diff --git a/x b/z\nsimilarity index 100%\nrename from x\nrename to z\n";
    let diff = read_unified_diff(text).expect("a well-formed diff");
    let mut files = tree(&[("x", &["a"])]);
    apply_unified_diff(&diff, &mut files).expect("the rename applies");
    assert_eq!(files, tree(&[("z", &["a"])]));
    let mut files = tree(&[("x", &["a"]), ("z", &["b"])]);
    assert_eq!(apply_unified_diff(&diff, &mut files).expect_err("z exists").reason, "FILE_EXISTS");
}
