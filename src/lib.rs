//! SigmaOS svntogit-community component.
//!
//! Rust implementation of `archlinux/svntogit-community` (and shared with
//! `svntogit-community`): tooling that mirrors the Arch packages SVN tree
//! into per-package git repositories.
//!
//! Upstream layout: `<pkgname>/trunk/` holds the development package, and
//! `<pkgname>/repos/<repo>-<arch>/` holds release variants. This port
//! implements the core operations the svntogit tooling performs:
//!
//! - scanning the tree into a package list ([`PackageEntry`])
//! - computing which `repos/*` directories exist per package
//! - generating the sync plan between an SVN checkout and git checkouts
//!   (file additions/modifications/deletions per branch)
//! - PKGBUILD `pkgver()`/`pkgrel` bump bookkeeping
//!
//! It deliberately does not shell out to `svn`/`git`; it operates on plain
//! directory trees so it can be tested without VCS binaries.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod error;
pub mod layout;
pub mod pkgbuild;
pub mod sync;

pub use error::SvnGitCommunityCommunityError;
pub use layout::{PackageEntry, RepoVariant, scan_packages, scan_tree};
pub use pkgbuild::{PkgbuildInfo, parse_pkgbuild};
pub use sync::{FileChange, SyncPlan, plan_sync};
