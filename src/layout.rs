//! Package-tree layout scanning.
//!
//! An svntogit packages checkout contains one directory per package:
//!
//! ```text
//! <pkgname>/
//!   trunk/            development package files
//!   repos/
//!     <repo>-<arch>/  release variant (e.g. extra-x86_64)
//! ```
//!
//! Each variant dir may also contain a `keys/` subdirectory (PGP keys).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::SvnGitCommunityCommunityError;

/// A release variant directory (`repos/<repo>-<arch>`).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RepoVariant {
    /// Repository name (e.g. `extra`, `core`, `multilib`).
    pub repo: String,
    /// Architecture (e.g. `x86_64`, `any`).
    pub arch: String,
    /// Path relative to the package dir.
    pub rel_path: String,
}

impl RepoVariant {
    /// `<repo>-<arch>` directory name.
    pub fn dir_name(&self) -> String {
        format!("{}-{}", self.repo, self.arch)
    }
}

/// A scanned package directory.
#[derive(Debug, Clone)]
pub struct PackageEntry {
    /// Package directory name (usually == pkgname).
    pub name: String,
    /// Absolute path to the package dir.
    pub path: PathBuf,
    /// Whether `trunk/` exists.
    pub has_trunk: bool,
    /// Release variants found under `repos/`.
    pub variants: Vec<RepoVariant>,
}

impl PackageEntry {
    /// All package file roots: `trunk` (if present) plus each variant dir.
    pub fn file_roots(&self) -> Vec<PathBuf> {
        let mut roots = Vec::new();
        if self.has_trunk {
            roots.push(self.path.join("trunk"));
        }
        for v in &self.variants {
            roots.push(self.path.join(&v.rel_path));
        }
        roots
    }
}

/// Scan a package tree root, returning all package entries sorted by name.
pub fn scan_packages(root: &Path) -> Result<Vec<PackageEntry>, SvnGitCommunityCommunityError> {
    if !root.is_dir() {
        return Err(SvnGitCommunityCommunityError::InvalidTree(format!(
            "{} is not a directory",
            root.display()
        )));
    }
    let mut entries = Vec::new();
    for entry in std::fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        // Skip hidden/VCS dirs.
        if name.starts_with('.') {
            continue;
        }
        if let Some(pkg) = scan_package(&path, &name)? {
            entries.push(pkg);
        }
    }
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(entries)
}

/// Scan the tree and index by package name.
pub fn scan_tree(root: &Path) -> Result<BTreeMap<String, PackageEntry>, SvnGitCommunityCommunityError> {
    Ok(scan_packages(root)?
        .into_iter()
        .map(|p| (p.name.clone(), p))
        .collect())
}

/// Scan a single package dir; returns None if it has no trunk and no repos.
fn scan_package(path: &Path, name: &str) -> Result<Option<PackageEntry>, SvnGitCommunityCommunityError> {
    let has_trunk = path.join("trunk").is_dir();
    let mut variants = Vec::new();
    let repos_dir = path.join("repos");
    if repos_dir.is_dir() {
        for entry in std::fs::read_dir(&repos_dir)? {
            let entry = entry?;
            let vpath = entry.path();
            if !vpath.is_dir() {
                continue;
            }
            let dirname = entry.file_name().to_string_lossy().to_string();
            // Expect "<repo>-<arch>"; arch may itself contain '-'? No —
            // split on the LAST '-' won't work for "extra-x86_64" (one dash).
            // Use first '-' as repo/arch separator.
            if let Some((repo, arch)) = dirname.split_once('-') {
                if !repo.is_empty() && !arch.is_empty() {
                    variants.push(RepoVariant {
                        repo: repo.to_string(),
                        arch: arch.to_string(),
                        rel_path: format!("repos/{dirname}"),
                    });
                }
            }
        }
    }
    variants.sort();
    if !has_trunk && variants.is_empty() {
        return Ok(None);
    }
    Ok(Some(PackageEntry {
        name: name.to_string(),
        path: path.to_path_buf(),
        has_trunk,
        variants,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mkpkg(root: &Path, name: &str, trunk: bool, variants: &[&str]) {
        let dir = root.join(name);
        std::fs::create_dir_all(&dir).unwrap();
        if trunk {
            std::fs::create_dir_all(dir.join("trunk")).unwrap();
            std::fs::write(dir.join("trunk/PKGBUILD"), "pkgname=x\n").unwrap();
        }
        for v in variants {
            let vd = dir.join("repos").join(v);
            std::fs::create_dir_all(&vd).unwrap();
            std::fs::write(vd.join("PKGBUILD"), "pkgname=x\n").unwrap();
        }
    }

    #[test]
    fn test_scan_packages() {
        let dir = tempfile::tempdir().unwrap();
        mkpkg(dir.path(), "foo", true, &["extra-x86_64"]);
        mkpkg(dir.path(), "bar", false, &["core-any", "multilib-x86_64"]);
        mkpkg(dir.path(), "empty", false, &[]);
        let pkgs = scan_packages(dir.path()).unwrap();
        assert_eq!(pkgs.len(), 2); // 'empty' has no trunk/repos → skipped
        assert_eq!(pkgs[0].name, "bar");
        assert_eq!(pkgs[0].variants.len(), 2);
        assert!(pkgs[1].has_trunk);
    }

    #[test]
    fn test_variant_dir_name() {
        let v = RepoVariant {
            repo: "extra".into(),
            arch: "x86_64".into(),
            rel_path: "repos/extra-x86_64".into(),
        };
        assert_eq!(v.dir_name(), "extra-x86_64");
    }

    #[test]
    fn test_file_roots() {
        let e = PackageEntry {
            name: "x".into(),
            path: PathBuf::from("/t/x"),
            has_trunk: true,
            variants: vec![RepoVariant {
                repo: "extra".into(),
                arch: "x86_64".into(),
                rel_path: "repos/extra-x86_64".into(),
            }],
        };
        let roots = e.file_roots();
        assert_eq!(roots.len(), 2);
        assert!(roots[0].ends_with("trunk"));
        assert!(roots[1].ends_with("extra-x86_64"));
    }
}
