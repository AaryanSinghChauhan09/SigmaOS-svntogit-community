//! Error types for the svntogit-community component.

/// Errors that can occur while scanning or planning svn→git syncs.
#[derive(Debug, thiserror::Error)]
pub enum SvnGitCommunityCommunityError {
    /// The package tree root does not look like an svntogit checkout.
    #[error("invalid package tree: {0}")]
    InvalidTree(String),

    /// A PKGBUILD could not be parsed.
    #[error("PKGBUILD parse error in {0}: {1}")]
    PkgbuildParse(String, String),

    /// I/O error.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}
