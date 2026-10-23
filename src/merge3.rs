//! Three-way merge of two edits of one base text.

use crate::myers::diff;
use crate::patch::Delta;

/// The merged lines and the number of conflict blocks among them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Merge3 {
    pub lines: Vec<String>,
    pub conflicts: usize,
}

/// Merge `ours` and `theirs`, two edits of `base`: the deltas of both greedy diffs from `base` are grouped into
/// clusters of source ranges that overlap or touch; a cluster changed by one side takes that side's lines, a
/// cluster both sides changed alike takes them once, and any other cluster becomes a conflict block.
pub fn merge3(base: &[String], ours: &[String], theirs: &[String]) -> Merge3 {
    let ours_patch = diff(base, ours);
    let theirs_patch = diff(base, theirs);
    let mut all: Vec<(usize, &Delta)> = ours_patch.deltas().iter().map(|d| (0, d)).collect();
    all.extend(theirs_patch.deltas().iter().map(|d| (1, d)));
    all.sort_by_key(|(side, d)| (d.source.position, *side));

    let mut merged = Merge3 { lines: Vec::new(), conflicts: 0 };
    let mut copied = 0;
    let mut next = 0;
    while next < all.len() {
        let start = all[next].1.source.position;
        let mut end = start + all[next].1.source.size();
        let mut cluster = vec![all[next]];
        next += 1;
        while next < all.len() && all[next].1.source.position <= end {
            end = end.max(all[next].1.source.position + all[next].1.source.size());
            cluster.push(all[next]);
            next += 1;
        }
        merged.lines.extend_from_slice(&base[copied..start]);
        copied = end;
        let side = |which: usize| -> Option<Vec<String>> {
            let deltas: Vec<&Delta> = cluster.iter().filter(|(s, _)| *s == which).map(|(_, d)| *d).collect();
            (!deltas.is_empty()).then(|| side_version(base, start, end, &deltas))
        };
        match (side(0), side(1)) {
            (Some(lines), None) | (None, Some(lines)) => merged.lines.extend(lines),
            (Some(ours), Some(theirs)) if ours == theirs => merged.lines.extend(ours),
            (Some(ours), Some(theirs)) => {
                merged.conflicts += 1;
                merged.lines.push("<<<<<<< ours".to_string());
                merged.lines.extend(ours);
                merged.lines.push("||||||| base".to_string());
                merged.lines.extend_from_slice(&base[start..end]);
                merged.lines.push("=======".to_string());
                merged.lines.extend(theirs);
                merged.lines.push(">>>>>>> theirs".to_string());
            }
            (None, None) => {}
        }
    }
    merged.lines.extend_from_slice(&base[copied..]);
    merged
}

/// `base[start..end]` with one side's deltas of the cluster applied.
fn side_version(base: &[String], start: usize, end: usize, deltas: &[&Delta]) -> Vec<String> {
    let mut out = Vec::new();
    let mut at = start;
    for delta in deltas {
        out.extend_from_slice(&base[at..delta.source.position]);
        out.extend(delta.target.lines.iter().cloned());
        at = delta.source.position + delta.source.size();
    }
    out.extend_from_slice(&base[at..end]);
    out
}
