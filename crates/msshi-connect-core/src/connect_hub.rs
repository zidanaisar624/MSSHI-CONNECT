use crate::constants::BOTS;
use crate::safety::atomic_write_acl_safe;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    env,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedApp {
    pub id: String,
    pub label: String,
    pub kind: String, // "local_cli" | "desktop_app" | "web_route"
    pub status: String, // "TERDETEKSI" | "BELUM TERPASANG" | "DIBATASI DI MODE LIGHT"
    pub executable: Option<String>,
    pub recommended_bots: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectHubState {
    pub auto_open_on_boot: bool, // Permanently false!
    pub zero_llm_calls: bool,
    pub browser_preference: String, // "none" | "chrome" | "edge" | "firefox"
    pub browser_profile_label: String,
    pub apps: Vec<DetectedApp>,
    pub bot_mappings: BTreeMap<String, Vec<String>>,
}

fn find_on_path(bin_names: &[&str]) -> Option<PathBuf> {
    let paths = env::var_os("PATH")?;
    for dir in env::split_paths(&paths) {
        if !dir.is_absolute() {
            continue;
        }
        for name in bin_names {
            let cand = dir.join(name);
            let is_store_alias = cfg!(windows)
                && cand
                    .parent()
                    .and_then(Path::file_name)
                    .is_some_and(|d| d.eq_ignore_ascii_case("WindowsApps"));
            if !is_store_alias && cand.is_file() {
                return Some(cand);
            }
        }
    }
    None
}

pub fn scan_connect_hub(light_mode: bool) -> ConnectHubState {
    let defs: &[(&str, &str, &str, &[&str], &[&str], bool)] = &[
        ("git", "Git CLI", "local_cli", &["git.exe", "git"], &["shikaku", "val"], false),
        ("python", "Python 3", "local_cli", &["python.exe", "python3"], &["shikaku", "val", "shizuka"], false),
        ("node", "Node.js", "local_cli", &["node.exe", "node"], &["shikaku", "kiki-chii"], false),
        ("ffmpeg", "FFmpeg Media", "local_cli", &["ffmpeg.exe", "ffmpeg"], &["kiki-chii"], false),
        ("blender-mcp", "Blender 3D (via Opt-in MCP for Blender)", "mcp_opt_in", &["blender.exe", "blender"], &["kiki-chii"], true),
        ("higgsfield", "Higgsfield CLI", "local_cli", &["higgsfield.ps1", "higgsfield.cmd", "higgsfield"], &["kiki-chii"], false),
        ("obsidian-bridge", "Obsidian 3-Zone Bridge", "local_cli", &["msshi-obsidian.cmd", "msshi-obsidian"], &["yukino", "shizuka", "val", "kiki-chii", "shikaku"], false),
        ("perplexity", "Perplexity Web Route", "web_route", &[], &["val", "yukino"], false),
        ("youtube-chrome", "YouTube Research Route", "web_route", &[], &["kiki-chii", "val"], false),
        ("whatsapp-web", "WhatsApp Web Route", "web_route", &[], &["yukino", "shizuka"], true),
    ];

    let mut apps = Vec::new();
    let mut bot_mappings: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for b in BOTS {
        bot_mappings.insert((*b).to_string(), Vec::new());
    }

    for (id, label, kind, bins, rec_bots, restricted_in_light) in defs {
        let exe = if bins.is_empty() {
            None
        } else {
            find_on_path(bins)
        };
        let status = if light_mode && *restricted_in_light {
            "DIBATASI DI MODE LIGHT".to_string()
        } else if *kind == "web_route" || exe.is_some() {
            "TERDETEKSI".to_string()
        } else {
            "BELUM TERPASANG".to_string()
        };

        if status == "TERDETEKSI" {
            for bot in *rec_bots {
                if let Some(list) = bot_mappings.get_mut(*bot) {
                    list.push((*id).to_string());
                }
            }
        }

        apps.push(DetectedApp {
            id: (*id).to_string(),
            label: (*label).to_string(),
            kind: (*kind).to_string(),
            status,
            executable: exe.map(|p| p.display().to_string()),
            recommended_bots: rec_bots.iter().map(|s| (*s).to_string()).collect(),
        });
    }

    ConnectHubState {
        auto_open_on_boot: false,
        zero_llm_calls: true,
        browser_preference: "none".to_string(),
        browser_profile_label: String::new(),
        apps,
        bot_mappings,
    }
}

pub fn save_connect_hub_prefs(
    msshi_home: &Path,
    mut state: ConnectHubState,
) -> Result<ConnectHubState, String> {
    // Hard security invariant: auto_open_on_boot is ALWAYS false, 0 cookies read.
    state.auto_open_on_boot = false;
    state.zero_llm_calls = true;
    let path = msshi_home.join("connect-hub-mapping.json");
    let raw = serde_json::to_vec_pretty(&state).map_err(|e| e.to_string())?;
    atomic_write_acl_safe(&path, &raw).map_err(|e| e.to_string())?;
    Ok(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connect_hub_enforces_auto_open_on_boot_false() {
        let mut hub = scan_connect_hub(true);
        assert!(!hub.auto_open_on_boot);
        hub.auto_open_on_boot = true; // Attempt to toggle must be clamped back to false
        let dir = std::env::temp_dir().join(format!("msshi-hub-test-{}", std::process::id()));
        let saved = save_connect_hub_prefs(&dir, hub).unwrap();
        assert!(!saved.auto_open_on_boot);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
