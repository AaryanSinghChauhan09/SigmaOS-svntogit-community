//! Sync planning between an SVN checkout tree and git working copies.
//!
//! For each package file root (`trunk/` or `repos/<repo>-<arch>/`), compute
//! the set of file changes needed to bring a git checkout in line with the
//! source tree: additions, modifications, deletions. This mirrors what the
//! upstream `svntogit` runner does with `svn export` + `git add -A`.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// Kind of file change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileChange {
    /// File exists in source but not in target.
    Added(PathBuf),
    /// File exists in both but content differs.
    Modified(PathBuf),
    /// File exists in target but not in source.
    Deleted(PathBuf),
}

/// Sync plan for one package variant (trunk or repos/<repo>-<arch>).
#[derive(Debug, Clone)]
pub struct SyncPlan {
    /// Variant name (`trunk` or `extra-x86_64`).
    pub variant: String,
    /// Changes needed, sorted by path.
    pub changes: Vec<FileChange>,
    /// Files identical in both trees.
    pub unchanged: usize,
}

impl SyncPlan {
    /// Whether the plan is empty (nothing to commit).
    pub fn is_clean(&self) -> bool {
        self.changes.is_empty()
    }

    /// Count changes by kind.
    pub fn count(&self, kind: &str) -> usize {
        self.changes
            .iter()
            .filter(|c| match (c, kind) {
                (FileChange::Added(_), "added") => true,
                (FileChange::Modified(_), "modified") => true,
                (FileChange::Deleted(_), "deleted") => true,
                _ => false,
            })
            .count()
    }
}

/// Recursively list all files under `root` as relative paths.
fn list_files(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut out = BTreeMap::new();
    if !root.is_dir() {
        return out;
    }
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for e in entries.flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else if p.is_file() {
                    let rel = p.strip_prefix(root).unwrap_or(&p).to_path_buf();
                    if let Ok(data) = std::fs::read(&p) {
                        out.insert(rel, data);
                    }
                }
            }
        }
    }
    out
}

/// Compute the sync plan between `source` (SVN export) and `target` (git
/// checkout) for a single variant directory.
pub fn plan_sync(source: &Path, target: &Path, variant: &str) -> SyncPlan {
    let src = list_files(source);
    let tgt = list_files(target);
    let mut changes = Vec::new();
    let mut unchanged = 0usize;

    for (rel, sdata) in &src {
        match tgt.get(rel) {
            None => changes.push(FileChange::Added(rel.clone())),
            Some(tdata) if tdata == sdata => unchanged += 1,
            Some(_) => changes.push(FileChange::Modified(rel.clone())),
        }
    }
    for rel in tgt.keys() {
        if !src.contains_key(rel) {
            changes.push(FileChange::Deleted(rel.clone()));
        }
    }
    changes.sort_by(|a, b| path_of(a).cmp(path_of(b)));

    SyncPlan {
        variant: variant.to_string(),
        changes,
        unchanged,
    }
}

fn path_of(c: &FileChange) -> &Path {
    match c {
        FileChange::Added(p) | FileChange::Modified(p) | FileChange::Deleted(p) => p,
    }
}

/// Plan syncs for every file root of a package entry.
///
/// `source_pkg_dir` = package dir in the SVN checkout;
/// `target_pkg_dir` = corresponding dir in the git checkout.
pub fn plan_package_sync(
    source_pkg_dir: &Path,
    target_pkg_dir: &Path,
    roots: &[(String, PathBuf)],
) -> Vec<SyncPlan> {
    roots
        .iter()
        .map(|(variant, rel)| {
            plan_sync(
                &source_pkg_dir.join(rel),
                &target_pkg_dir.join(rel),
                variant,
            )
        })
        .collect()
}

/// All changed paths across plans, deduplicated.
pub fn all_changes(plans: &[SyncPlan]) -> BTreeSet<PathBuf> {
    let mut out = BTreeSet::new();
    for p in plans {
        for c in &p.changes {
            out.insert(path_of(c).to_path_buf());
        }
    }
    out
}
