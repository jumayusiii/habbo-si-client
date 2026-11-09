use serde::Deserialize;
use std::time::Duration;

use crate::settings::{RELEASE_TAG, VERSION};

const API_ROOT: &str = "https://api.github.com/repos/jumayusiii/habbo-si-client/";

#[derive(Clone, Debug)]
pub struct UpdateInfo {
    pub latest: String,
    pub tag: String,
    pub page: String,
}

impl UpdateInfo {
    pub fn display(&self) -> &str {
        if self.tag.is_empty() {
            &self.latest
        } else {
            &self.tag
        }
    }
}

#[derive(Clone, Debug)]
pub struct ReleaseNotes {
    pub tag: String,
    pub name: String,
    pub body: String,
    pub page: String,
}

#[derive(Debug, Deserialize)]
struct Release {
    #[serde(default)]
    tag_name: String,
    #[serde(default)]
    html_url: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
}

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(8))
        .user_agent(&format!("HabboSiClientRust/{VERSION}"))
        .build()
}

fn get_json<T: serde::de::DeserializeOwned>(url: &str) -> Option<T> {
    let response = agent()
        .get(url)
        .set("Accept", "application/vnd.github+json")
        .call()
        .ok()?;
    response.into_json::<T>().ok()
}

pub fn latest(beta: bool) -> Option<UpdateInfo> {
    if beta { latest_beta() } else { latest_stable() }
}

fn latest_stable() -> Option<UpdateInfo> {
    let release = get_json::<Release>(&format!("{API_ROOT}releases/latest"))?;
    if release.tag_name.is_empty() || release.html_url.is_empty() {
        return None;
    }
    let numeric = clean_tag(&release.tag_name);
    Some(UpdateInfo {
        latest: numeric.clone(),
        tag: numeric,
        page: release.html_url,
    })
}

fn latest_beta() -> Option<UpdateInfo> {
    let releases = get_json::<Vec<Release>>(&format!("{API_ROOT}releases?per_page=30"))?;
    let mut best: Option<UpdateInfo> = None;
    for release in releases {
        if release.draft || !release.prerelease {
            continue;
        }
        if release.tag_name.is_empty() || release.html_url.is_empty() {
            continue;
        }
        let info = UpdateInfo {
            latest: clean_tag(&release.tag_name),
            tag: normalize_tag(&release.tag_name),
            page: release.html_url,
        };
        if best
            .as_ref()
            .map(|b| is_newer_beta(&info.tag, &b.tag))
            .unwrap_or(true)
        {
            best = Some(info);
        }
    }
    best
}

pub fn has_update(info: &UpdateInfo, beta: bool) -> bool {
    if beta {
        is_newer_beta(&info.tag, VERSION)
    } else {
        is_newer(&info.latest, VERSION)
    }
}

pub fn notes(tag: &str) -> Option<ReleaseNotes> {
    if tag.is_empty() {
        return None;
    }
    let release = get_json::<Release>(&format!(
        "{API_ROOT}releases/tags/v{}",
        tag.trim_start_matches('v')
    ))?;
    let body = release.body.unwrap_or_default();
    let tag = normalize_tag(&release.tag_name);
    if tag.is_empty() || body.trim().is_empty() {
        return None;
    }
    Some(ReleaseNotes {
        tag,
        name: if release.name.is_empty() {
            format!("Habbo.si Client v{VERSION}")
        } else {
            release.name
        },
        body,
        page: release.html_url,
    })
}

pub fn current_notes() -> Option<ReleaseNotes> {
    notes(RELEASE_TAG)
}

pub fn normalize_tag(tag: &str) -> String {
    tag.trim()
        .strip_prefix('v')
        .or_else(|| tag.trim().strip_prefix('V'))
        .unwrap_or(tag.trim())
        .to_string()
}

pub fn clean_tag(tag: &str) -> String {
    let normalized = normalize_tag(tag);
    normalized
        .split_once('-')
        .map(|(base, _)| base)
        .unwrap_or(&normalized)
        .trim()
        .to_string()
}

fn parts(version: &str) -> [u64; 4] {
    let normalized = normalize_tag(version);
    let base = normalized
        .split_once('-')
        .map(|(b, _)| b)
        .unwrap_or(&normalized);
    let mut out = [0u64; 4];
    for (i, part) in base.split('.').take(4).enumerate() {
        out[i] = part.parse().unwrap_or(0);
    }
    out
}

fn ordinal(version: &str) -> u64 {
    let normalized = normalize_tag(version);
    let Some((_, suffix)) = normalized.split_once('-') else {
        return 0;
    };
    suffix
        .rsplit('.')
        .next()
        .and_then(|n| n.parse().ok())
        .unwrap_or(0)
}

pub fn is_newer(latest: &str, running: &str) -> bool {
    parts(latest) > parts(running)
}

pub fn is_newer_beta(latest: &str, running: &str) -> bool {
    let a = normalize_tag(latest);
    let b = normalize_tag(running);
    if a.is_empty() || a.eq_ignore_ascii_case(&b) {
        return false;
    }
    let pa = parts(&a);
    let pb = parts(&b);
    if pa != pb {
        return pa > pb;
    }
    ordinal(&a) > ordinal(&b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_v_and_suffix_for_stable() {
        assert_eq!(clean_tag("v1.1.6"), "1.1.6");
        assert_eq!(clean_tag("V1.1.5-hotfix2"), "1.1.5");
    }

    #[test]
    fn orders_beta_ordinals() {
        assert!(is_newer_beta("1.2.0-beta.2", "1.2.0-beta.1"));
        assert!(!is_newer_beta("1.2.0-beta.1", "1.2.0-beta.1"));
        assert!(is_newer_beta("1.2.0-beta.1", "1.1.5"));
        assert!(!is_newer_beta("1.1.5", "1.2.0-beta.1"));
    }

    #[test]
    fn stable_compare_ignores_suffix() {
        assert!(is_newer("1.1.6", "1.1.5"));
        assert!(!is_newer("1.1.5-hotfix2", "1.1.5"));
    }
}
