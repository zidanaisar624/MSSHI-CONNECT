use crate::constants::{APP_VERSION, DEFAULT_REF, UPSTREAM_FULL_NAME};
use crate::safety::{atomic_write_acl_safe, validate_github_url};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseRef {
    pub name: String,
    pub kind: String, // "release" | "tag" | "branch"
    pub commit_sha: Option<String>,
    pub published_at: Option<String>,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubCatalog {
    pub repo: String,
    pub connection_status: String, // ONLINE_OK | ONLINE_ETAG_CACHED | RATE_LIMITED | OFFLINE
    pub from_cache: bool,
    pub rate_limit_remaining: Option<i64>,
    pub latest_stable: String,
    pub main_commit_sha: Option<String>,
    pub refs: Vec<ReleaseRef>,
    pub changelog_preview: String,
    pub error_message: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CachedPayload {
    etag: Option<String>,
    body: serde_json::Value,
}

pub struct GitHubClient {
    cache_dir: PathBuf,
    agent: ureq::Agent,
}

impl GitHubClient {
    pub fn new(cache_dir: PathBuf) -> Self {
        let agent = ureq::AgentBuilder::new()
            .timeout_connect(Duration::from_secs(8))
            .timeout_read(Duration::from_secs(12))
            .user_agent(&format!("MSSHI-Connect/{APP_VERSION}"))
            .build();
        Self { cache_dir, agent }
    }

    fn cache_file(&self, key: &str) -> PathBuf {
        let safe: String = key
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
            .collect();
        self.cache_dir.join(format!("{safe}.json"))
    }

    fn get_json_etag(&self, url: &str, cache_key: &str) -> Result<(serde_json::Value, bool, Option<i64>), String> {
        let safe_url = validate_github_url(url).map_err(|e| e.to_string())?;
        let cf = self.cache_file(cache_key);
        let cached: Option<CachedPayload> = fs::read_to_string(&cf)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok());

        let mut req = self.agent.get(&safe_url).set("Accept", "application/vnd.github+json");
        if let Ok(token) = std::env::var("GITHUB_TOKEN") {
            if !token.trim().is_empty() {
                req = req.set("Authorization", &format!("Bearer {}", token.trim()));
            }
        }
        if let Some(ref c) = cached {
            if let Some(ref etag) = c.etag {
                req = req.set("If-None-Match", etag);
            }
        }

        match req.call() {
            Ok(resp) => {
                let remaining = resp
                    .header("x-ratelimit-remaining")
                    .and_then(|s| s.parse::<i64>().ok());
                if resp.status() == 304 {
                    if let Some(c) = cached {
                        return Ok((c.body, true, remaining));
                    }
                }
                let etag = resp.header("etag").map(|s| s.to_string());
                let body: serde_json::Value = resp.into_json().map_err(|e| e.to_string())?;
                let payload = CachedPayload { etag, body: body.clone() };
                if let Ok(raw) = serde_json::to_vec_pretty(&payload) {
                    let _ = atomic_write_acl_safe(&cf, &raw);
                }
                Ok((body, false, remaining))
            }
            Err(ureq::Error::Status(304, resp)) => {
                let remaining = resp
                    .header("x-ratelimit-remaining")
                    .and_then(|s| s.parse::<i64>().ok());
                if let Some(c) = cached {
                    return Ok((c.body, true, remaining));
                }
                Err("HTTP 304 tanpa cache lokal".to_string())
            }
            Err(ureq::Error::Status(403, _)) => Err("RATE_LIMITED".to_string()),
            Err(err) => Err(err.to_string()),
        }
    }

    pub fn fetch_catalog(&self) -> GitHubCatalog {
        let tags_url = format!("https://api.github.com/repos/{UPSTREAM_FULL_NAME}/tags?per_page=20");
        let main_url = format!("https://api.github.com/repos/{UPSTREAM_FULL_NAME}/commits/main");

        let mut refs = Vec::new();
        let mut from_cache = false;
        let mut rate_remaining = None;
        let mut latest_stable = DEFAULT_REF.to_string();

        let tags_res = self.get_json_etag(&tags_url, "tags");
        let main_res = self.get_json_etag(&main_url, "main_commit");

        match tags_res {
            Ok((val, cached, rem)) => {
                from_cache |= cached;
                rate_remaining = rem;
                if let Some(arr) = val.as_array() {
                    for (idx, item) in arr.iter().enumerate() {
                        if let Some(name) = item.get("name").and_then(|s| s.as_str()) {
                            if idx == 0 {
                                latest_stable = name.to_string();
                            }
                            let sha = item
                                .get("commit")
                                .and_then(|c| c.get("sha"))
                                .and_then(|s| s.as_str())
                                .map(|s| s.to_string());
                            refs.push(ReleaseRef {
                                name: name.to_string(),
                                kind: "tag".to_string(),
                                commit_sha: sha,
                                published_at: None,
                                summary: format!("Tag rilis stabil {name}"),
                            });
                        }
                    }
                }
            }
            Err(err) => {
                let status = if err == "RATE_LIMITED" { "RATE_LIMITED" } else { "OFFLINE" };
                return GitHubCatalog {
                    repo: UPSTREAM_FULL_NAME.to_string(),
                    connection_status: status.to_string(),
                    from_cache: false,
                    rate_limit_remaining: if status == "RATE_LIMITED" { Some(0) } else { None },
                    latest_stable,
                    main_commit_sha: None,
                    refs: vec![ReleaseRef {
                        name: DEFAULT_REF.to_string(),
                        kind: "tag".to_string(),
                        commit_sha: None,
                        published_at: None,
                        summary: "Baseline default offline (pilih folder/ZIP lokal bila offline)".to_string(),
                    }],
                    changelog_preview: "Mode offline: tidak dapat mengambil CHANGELOG.md dari GitHub.".to_string(),
                    error_message: Some(err),
                };
            }
        }

        let main_sha = match main_res {
            Ok((val, cached, rem)) => {
                from_cache |= cached;
                if rem.is_some() {
                    rate_remaining = rem;
                }
                let sha = val.get("sha").and_then(|s| s.as_str()).map(|s| s.to_string());
                let msg = val
                    .get("commit")
                    .and_then(|c| c.get("message"))
                    .and_then(|s| s.as_str())
                    .and_then(|m| m.lines().next())
                    .unwrap_or("Branch main terbaru");
                refs.push(ReleaseRef {
                    name: "main".to_string(),
                    kind: "branch".to_string(),
                    commit_sha: sha.clone(),
                    published_at: val
                        .get("commit")
                        .and_then(|c| c.get("committer"))
                        .and_then(|c| c.get("date"))
                        .and_then(|s| s.as_str())
                        .map(|s| s.to_string()),
                    summary: format!("Rolling main: {msg}"),
                });
                sha
            }
            Err(_) => None,
        };

        let changelog_url = format!("https://raw.githubusercontent.com/{UPSTREAM_FULL_NAME}/{latest_stable}/CHANGELOG.md");
        let changelog_preview = validate_github_url(&changelog_url)
            .ok()
            .and_then(|u| self.agent.get(&u).call().ok())
            .and_then(|r| r.into_string().ok())
            .map(|s| s.chars().take(3500).collect())
            .unwrap_or_else(|| "Ringkasan changelog tersedia di GitHub Releases.".to_string());

        GitHubCatalog {
            repo: UPSTREAM_FULL_NAME.to_string(),
            connection_status: if from_cache {
                "ONLINE_ETAG_CACHED".to_string()
            } else {
                "ONLINE_OK".to_string()
            },
            from_cache,
            rate_limit_remaining: rate_remaining,
            latest_stable,
            main_commit_sha: main_sha,
            refs,
            changelog_preview,
            error_message: None,
        }
    }

    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }
}
