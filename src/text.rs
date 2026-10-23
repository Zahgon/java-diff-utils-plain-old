//! Differences over whole texts: line diffs of `\n`-separated text and character-level inline diffs.

use crate::java;
use crate::myers::diff;
use crate::patch::Patch;

fn lines(text: &str) -> Vec<String> {
    java::split(text, "\n").into_iter().map(str::to_string).collect()
}

/// Line differences between two texts split at `\n` the way `String.split("\n")` splits them.
pub fn diff_text(source: &str, target: &str) -> Patch {
    diff(&lines(source), &lines(target))
}

/// Character-level differences; each side of every delta is folded into one string, and positions
/// count characters.
pub fn diff_inline(original: &str, revised: &str) -> Patch {
    let split = |text: &str| -> Vec<String> { text.chars().map(String::from).collect() };
    let character_patch = diff(&split(original), &split(revised));
    let mut patch = Patch::new();
    for delta in character_patch.deltas() {
        let mut folded = delta.clone();
        folded.source.lines = fold(&delta.source.lines);
        folded.target.lines = fold(&delta.target.lines);
        patch.add_delta(folded);
    }
    patch
}

fn fold(characters: &[String]) -> Vec<String> {
    if characters.is_empty() { Vec::new() } else { vec![characters.concat()] }
}
