// SPDX-FileCopyrightText: 2026 Fatih Bucaklioglu
// SPDX-License-Identifier: GPL-3.0-or-later
//! GitHub Releases: typed parsing and explainable asset selection (ARCHITECTURE.md §5.3).

use crate::store::{valid_release_url, valid_tool_name, COMPAT_PREFIX};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug)]
pub struct Release {
    pub tag_name: String,
    #[serde(default)]
    pub published_at: Option<String>,
    #[serde(default)]
    pub prerelease: bool,
    #[serde(default)]
    pub draft: bool,
    #[serde(default)]
    pub assets: Vec<Asset>,
}

#[derive(Deserialize, Debug, PartialEq)]
pub struct Asset {
    pub name: String,
    #[serde(default)]
    pub size: u64,
    pub browser_download_url: String,
    /// "sha256:<hex>", computed by GitHub when the file was uploaded (absent on old releases).
    #[serde(default)]
    pub digest: Option<String>,
}

/// A catalogue card for a Steam compatibility tool, in the shape the QML catalogue uses.
#[derive(Serialize, Debug, PartialEq)]
pub struct CompatItem {
    pub id: String,
    pub name: String,
    pub category: String,
    pub summary: String,
    pub icon: String,
    pub source: String,
    pub url: String,
    pub sum: String,
    /// Archive size in bytes (the free-space check asks for 3× this).
    pub size: u64,
}

/// Why a release was or was not offered (kept for logs and tests).
#[derive(Debug, PartialEq)]
pub enum Choice<'a> {
    Offer {
        archive: &'a Asset,
        checksum: Option<&'a Asset>,
    },
    Skip(&'static str),
}

/// Asset rules, in order:
/// 1. drafts and prereleases are skipped;
/// 2. the tag must be a safe folder name (it is the folder the archive must create);
/// 3. archive: `*x86_64.tar.gz`, else any `*.tar.gz` that is not for aarch64/arm64;
/// 4. checksum: the archive name with `.tar.gz` replaced by `.sha512sum`, if published;
/// 5. both links must be GitHub release downloads.
pub fn choose(r: &Release) -> Choice<'_> {
    if r.draft {
        return Choice::Skip("draft");
    }
    if r.prerelease {
        return Choice::Skip("prerelease");
    }
    if !valid_tool_name(&r.tag_name) {
        return Choice::Skip("tag is not a safe folder name");
    }
    let not_arm = |n: &str| !n.contains("aarch64") && !n.contains("arm64");
    let archive = r.assets.iter().find(|a| a.name.ends_with("x86_64.tar.gz")).or_else(|| {
        r.assets
            .iter()
            .find(|a| a.name.ends_with(".tar.gz") && not_arm(&a.name))
    });
    let Some(archive) = archive else {
        return Choice::Skip("no x86_64 .tar.gz asset");
    };
    if !valid_release_url(&archive.browser_download_url) {
        return Choice::Skip("archive link is not a GitHub release download");
    }
    let sum_name = format!("{}.sha512sum", archive.name.trim_end_matches(".tar.gz"));
    let checksum = r
        .assets
        .iter()
        .find(|a| a.name == sum_name && valid_release_url(&a.browser_download_url));
    Choice::Offer { archive, checksum }
}

/// The newest `max` offerable Proton-GE releases as catalogue cards.
pub fn proton_ge_items(json: &str, max: usize) -> Result<Vec<CompatItem>, String> {
    let releases: Vec<Release> = serde_json::from_str(json).map_err(|e| format!("unexpected GitHub reply: {e}"))?;
    let mut out = Vec::new();
    for r in &releases {
        if out.len() >= max {
            break;
        }
        match choose(r) {
            Choice::Offer { archive, checksum } => {
                let date = r.published_at.as_deref().unwrap_or("").get(..10).unwrap_or("");
                out.push(CompatItem {
                    id: format!("{COMPAT_PREFIX}{}", r.tag_name),
                    name: r.tag_name.clone(),
                    category: "Compatibility".into(),
                    summary: format!(
                        "Proton with extra game fixes, by GloriousEggroll · released {date} · {} MB download",
                        (archive.size + 524_288) / 1_048_576
                    ),
                    icon: String::new(),
                    source: "compat".into(),
                    url: archive.browser_download_url.clone(),
                    sum: checksum.map(|c| c.browser_download_url.clone()).unwrap_or_default(),
                    size: archive.size,
                });
            }
            Choice::Skip(why) => tracing::debug!(tag = %r.tag_name, why, "release not offered"),
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASE: &str = "https://github.com/GloriousEggroll/proton-ge-custom/releases/download";

    fn rel(tag: &str, pre: bool, names: &[&str]) -> String {
        let assets: Vec<String> = names
            .iter()
            .map(|n| format!(r#"{{"name": "{n}", "size": 566859228, "browser_download_url": "{BASE}/{tag}/{n}"}}"#))
            .collect();
        format!(
            r#"{{"tag_name": "{tag}", "published_at": "2026-11-03T07:19:38Z", "prerelease": {pre}, "assets": [{}]}}"#,
            assets.join(", ")
        )
    }

    #[test]
    fn picks_x86_64_archive_and_its_checksum() {
        let json = format!(
            "[{}]",
            rel(
                "GE-Proton11-12",
                false,
                &[
                    "GE-Proton11-12-aarch64.tar.gz",
                    "GE-Proton11-12-aarch64.sha512sum",
                    "GE-Proton11-12-x86_64.tar.gz",
                    "GE-Proton11-12-x86_64.sha512sum",
                ]
            )
        );
        let items = proton_ge_items(&json, 3).unwrap();
        assert_eq!(items.len(), 1);
        assert!(items[0].url.ends_with("GE-Proton11-12-x86_64.tar.gz"));
        assert!(items[0].sum.ends_with("GE-Proton11-12-x86_64.sha512sum"));
        assert_eq!(items[0].id, "compat:GE-Proton11-12");
        assert!(items[0].summary.contains("2026-11-03") && items[0].summary.contains("541 MB"));
    }

    #[test]
    fn older_naming_and_missing_checksum() {
        let json = format!("[{}]", rel("GE-Proton9-27", false, &["GE-Proton9-27.tar.gz"]));
        let items = proton_ge_items(&json, 3).unwrap();
        assert!(items[0].url.ends_with("GE-Proton9-27.tar.gz"));
        assert_eq!(items[0].sum, "", "no checksum published → shown as not verified");
    }

    #[test]
    fn skips_prereleases_arm_only_and_unsafe_tags() {
        let json = format!(
            "[{}, {}, {}, {}]",
            rel("GE-Proton12-1", true, &["GE-Proton12-1-x86_64.tar.gz"]),
            rel("GE-Proton11-13", false, &["GE-Proton11-13-aarch64.tar.gz"]),
            rel("../evil", false, &["x86_64.tar.gz"]),
            rel("GE-Proton11-12", false, &["GE-Proton11-12-x86_64.tar.gz"])
        );
        let items = proton_ge_items(&json, 3).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "GE-Proton11-12");
    }

    #[test]
    fn rejects_non_json() {
        assert!(proton_ge_items("<html>rate limited</html>", 3).is_err());
    }
}
