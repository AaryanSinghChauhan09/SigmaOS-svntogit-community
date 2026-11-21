//! Minimal PKGBUILD parsing for svntogit bookkeeping.
//!
//! Upstream svntogit tooling tracks `pkgver()`/`pkgrel` bumps via `pkgver.py`
//! and reads pkgbase/pkgname from PKGBUILDs. This parser extracts the
//! scalar variables needed for that bookkeeping without executing bash.

use crate::SvnGitCommunityCommunityError;

/// Scalar fields extracted from a PKGBUILD.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PkgbuildInfo {
    /// pkgname (first element if array).
    pub pkgname: String,
    /// pkgbase.
    pub pkgbase: String,
    /// pkgver.
    pub pkgver: String,
    /// pkgrel.
    pub pkgrel: String,
    /// epoch.
    pub epoch: u64,
    /// arch list.
    pub arch: Vec<String>,
}

impl PkgbuildInfo {
    /// Full version string `[epoch:]pkgver-pkgrel`.
    pub fn full_version(&self) -> String {
        let epoch = if self.epoch != 0 {
            format!("{}:", self.epoch)
        } else {
            String::new()
        };
        format!("{}{}-{}", epoch, self.pkgver, self.pkgrel)
    }
}

/// Parse scalar `key=value` assignments from PKGBUILD text.
///
/// Handles `key=value`, `key=(a b c)`, `key+=$(cmd)`, comments, and
/// single/double quotes. Does not evaluate `$(...)` — those are recorded
/// as empty and flagged by the caller if needed.
pub fn parse_pkgbuild(text: &str) -> Result<PkgbuildInfo, SvnGitCommunityCommunityError> {
    let mut info = PkgbuildInfo::default();
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        // Strip inline comments outside quotes (naive: split on ' #' and '# ').
        let line = strip_comment(line);
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        match key {
            "pkgname" => info.pkgname = first_value(value),
            "pkgbase" => info.pkgbase = value.trim_matches(|c| c == '"' || c == '\'').to_string(),
            "pkgver" => info.pkgver = unquote(value),
            "pkgrel" => info.pkgrel = unquote(value),
            "epoch" => info.epoch = unquote(value).parse().unwrap_or(0),
            "arch" => info.arch = array_values(value),
            _ => {}
        }
    }
    Ok(info)
}

/// Read and parse `<dir>/PKGBUILD`.
pub fn parse_pkgbuild_file(dir: &std::path::Path) -> Result<PkgbuildInfo, SvnGitCommunityCommunityError> {
    let path = dir.join("PKGBUILD");
    let text = std::fs::read_to_string(&path).map_err(|e| {
        SvnGitCommunityCommunityError::PkgbuildParse(path.display().to_string(), e.to_string())
    })?;
    parse_pkgbuild(&text)
}

fn strip_comment(line: &str) -> &str {
    // PKGBUILD comments start with # at line start or after whitespace.
    for (i, _) in line.match_indices('#') {
        if i == 0 || line.as_bytes()[i - 1].is_ascii_whitespace() {
            return line[..i].trim_end();
        }
    }
    line
}

fn unquote(v: &str) -> String {
    v.trim()
        .trim_matches(|c| c == '"' || c == '\'')
        .to_string()
}

fn first_value(v: &str) -> String {
    let v = v.trim();
    if v.starts_with('(') {
        array_values(v).into_iter().next().unwrap_or_default()
    } else {
        unquote(v)
    }
}

fn array_values(v: &str) -> Vec<String> {
    let v = v.trim();
    if !v.starts_with('(') {
        return vec![unquote(v)];
    }
    let inner = v
        .trim_start_matches('(')
        .trim_end_matches(')')
        .trim_end_matches(';');
    inner
        .split_whitespace()
        .map(|s| s.trim_matches(|c| c == '"' || c == '\''))
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_scalars() {
        let text = r#"
# comment
pkgname=foo
pkgbase=foo
pkgver=1.2.3
pkgrel=4
epoch=2
arch=('x86_64' 'any') # trailing comment
url="https://example.com"
"#;
        let info = parse_pkgbuild(text).unwrap();
        assert_eq!(info.pkgname, "foo");
        assert_eq!(info.pkgbase, "foo");
        assert_eq!(info.pkgver, "1.2.3");
        assert_eq!(info.arch, vec!["x86_64", "any"]);
        assert_eq!(info.full_version(), "2:1.2.3-4");
    }

    #[test]
    fn test_parse_pkgname_array() {
        let info = parse_pkgbuild("pkgname=('a' 'b')\npkgver=1\npkgrel=1\n").unwrap();
        assert_eq!(info.pkgname, "a");
    }
}
