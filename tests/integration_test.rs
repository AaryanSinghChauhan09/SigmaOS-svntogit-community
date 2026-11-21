//! Integration tests for sigmaos-svntogit-community.

use std::path::Path;

use svntogit_community::{
    FileChange, PackageEntry, RepoVariant, parse_pkgbuild, plan_sync, scan_packages,
};

fn mkpkg(root: &Path, name: &str, trunk: bool, variants: &[&str]) {
    let dir = root.join(name);
    std::fs::create_dir_all(&dir).unwrap();
    if trunk {
        std::fs::create_dir_all(dir.join("trunk")).unwrap();
        std::fs::write(dir.join("trunk/PKGBUILD"), "pkgname=x\npkgver=1\npkgrel=1\n").unwrap();
    }
    for v in variants {
        let vd = dir.join("repos").join(v);
        std::fs::create_dir_all(&vd).unwrap();
        std::fs::write(vd.join("PKGBUILD"), "pkgname=x\npkgver=1\npkgrel=1\n").unwrap();
    }
}

#[test]
fn test_scan_realistic_tree() {
    let dir = tempfile::tempdir().unwrap();
    mkpkg(dir.path(), "389-ds-base", true, &["extra-x86_64"]);
    mkpkg(dir.path(), "a2ps", true, &["extra-x86_64"]);
    let pkgs = scan_packages(dir.path()).unwrap();
    assert_eq!(pkgs.len(), 2);
    let a2ps = pkgs.iter().find(|p| p.name == "a2ps").unwrap();
    assert_eq!(a2ps.variants[0].repo, "extra");
    assert_eq!(a2ps.variants[0].arch, "x86_64");
}

#[test]
fn test_sync_plan_detects_changes() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("src");
    let tgt = dir.path().join("tgt");
    std::fs::create_dir_all(src.join("sub")).unwrap();
    std::fs::create_dir_all(&tgt).unwrap();
    std::fs::write(src.join("PKGBUILD"), "new\n").unwrap();
    std::fs::write(src.join("sub/file"), "data\n").unwrap();
    std::fs::write(tgt.join("PKGBUILD"), "old\n").unwrap();
    std::fs::write(tgt.join("stale"), "gone\n").unwrap();

    let plan = plan_sync(&src, &tgt, "trunk");
    assert_eq!(plan.count("added"), 1); // sub/file
    assert_eq!(plan.count("modified"), 1); // PKGBUILD
    assert_eq!(plan.count("deleted"), 1); // stale
    assert!(!plan.is_clean());
}

#[test]
fn test_sync_plan_clean_tree() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("src");
    let tgt = dir.path().join("tgt");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::create_dir_all(&tgt).unwrap();
    std::fs::write(src.join("a"), "same\n").unwrap();
    std::fs::write(tgt.join("a"), "same\n").unwrap();
    let plan = plan_sync(&src, &tgt, "trunk");
    assert!(plan.is_clean());
    assert_eq!(plan.unchanged, 1);
}

#[test]
fn test_parse_pkgbuild_upstream_style() {
    let text = "pkgname=foo\npkgbase=foo\npkgver=1.0\npkgrel=2\narch=('x86_64')\n";
    let info = parse_pkgbuild(text).unwrap();
    assert_eq!(info.full_version(), "1.0-2");
}

#[test]
fn test_package_entry_file_roots() {
    let e = PackageEntry {
        name: "p".into(),
        path: Path::new("/x/p").to_path_buf(),
        has_trunk: true,
        variants: vec![RepoVariant {
            repo: "extra".into(),
            arch: "any".into(),
            rel_path: "repos/extra-any".into(),
        }],
    };
    assert_eq!(e.file_roots().len(), 2);
}

#[test]
fn test_file_change_variants() {
    let c = FileChange::Added(Path::new("x").to_path_buf());
    assert!(matches!(c, FileChange::Added(_)));
}
