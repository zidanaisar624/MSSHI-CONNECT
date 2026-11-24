use crate::constants::DEFAULT_REF;
use crate::safety::atomic_write_acl_safe;
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FeatureCandidate {
    pub id: String,
    pub title: String,
    pub category: String, // "skill" | "addon" | "context_source" | "behavior"
    pub disk_impact: String,
    pub warning: String,
    pub default_checked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClassifiedUpdate {
    pub target_ref: String,
    /// Core system/security/bugfix files that auto-apply without prompting when Auto-Upgrade is ON.
    pub system_patches: Vec<String>,
    pub system_auto_apply_silent: bool,
    /// Optional features/skills/add-ons that trigger a warning and let the user choose Yes/No per feature.
    pub optional_features: Vec<FeatureCandidate>,
    pub requires_user_feature_prompt: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConnectPrefs {
    pub auto_upgrade_enabled: bool,
    /// "smart_split" (default: system=auto tanpa tanya, fitur=warning & pilih per fitur)
    /// | "notify_confirm" (tanya semua)
    /// | "auto_apply_zone1" (otomatis semua Zona 1)
    pub auto_upgrade_policy: String,
    /// "stable" | "main"
    pub channel: String,
    pub selected_ref: String,
    /// Feature IDs explicitly opted-in by the user during feature prompt
    pub approved_features: Vec<String>,
    /// Feature IDs explicitly skipped/declined by the user ("pilih ga untuk fitur tertentu")
    pub skipped_features: Vec<String>,
    pub local_source_path: Option<String>,
    pub last_checked_utc: Option<String>,
}

impl Default for ConnectPrefs {
    fn default() -> Self {
        Self {
            auto_upgrade_enabled: true,
            auto_upgrade_policy: "smart_split".to_string(),
            channel: "stable".to_string(),
            selected_ref: DEFAULT_REF.to_string(),
            approved_features: Vec::new(),
            skipped_features: Vec::new(),
            local_source_path: None,
            last_checked_utc: None,
        }
    }
}

impl ConnectPrefs {
    pub fn normalized(mut self) -> Self {
        if !matches!(
            self.auto_upgrade_policy.as_str(),
            "smart_split" | "notify_confirm" | "auto_apply_zone1"
        ) {
            self.auto_upgrade_policy = "smart_split".to_string();
        }
        if self.channel != "stable" && self.channel != "main" {
            self.channel = "stable".to_string();
        }
        if self.selected_ref.trim().is_empty() {
            self.selected_ref = DEFAULT_REF.to_string();
        }
        self
    }

    /// Resolves effective target ref respecting Auto-Upgrade ON/OFF and Manual Picker.
    pub fn resolve_target_ref(&self, latest_stable: &str) -> String {
        if !self.auto_upgrade_enabled {
            return self.selected_ref.clone();
        }
        if self.channel == "main" {
            return "main".to_string();
        }
        latest_stable.to_string()
    }
}

/// Classifies an incoming GitHub MSSHI update into:
/// 1) System patches (`scripts/`, `hermes_plugins/`, bugfixes, security guards) -> auto-updated silently without nagging!
/// 2) Optional/new features (`skills/`, `optional/`, heavy add-ons, context-sources) -> warns & asks user per feature!
pub fn classify_msshi_update(
    target_ref: &str,
    changed_paths: &[String],
    prefs: &ConnectPrefs,
) -> ClassifiedUpdate {
    let mut system_patches = Vec::new();
    let mut optional_features = Vec::new();

    for raw in changed_paths {
        let p = raw.replace('\\', "/");
        if p.starts_with("optional/") || p.starts_with("skills/") {
            let id = p.split('/').nth(1).unwrap_or(&p).to_string();
            if prefs.skipped_features.contains(&id) {
                continue;
            }
            if !optional_features.iter().any(|f: &FeatureCandidate| f.id == id) {
                let is_optional_dir = p.starts_with("optional/");
                optional_features.push(FeatureCandidate {
                    id: id.clone(),
                    title: format!("Fitur / Skill Baru: {id}"),
                    category: if is_optional_dir { "addon".into() } else { "skill".into() },
                    disk_impact: if is_optional_dir { "Opsional (~1–180 MB)".into() } else { "Ringan (< 1 MB)".into() },
                    warning: format!(
                        "Pembaruan ini menyertakan komponen '{id}'. Pilih apakah ingin memasang atau melewati fitur ini."
                    ),
                    default_checked: !is_optional_dir,
                });
            }
        } else {
            system_patches.push(p);
        }
    }

    let system_auto_apply_silent = prefs.auto_upgrade_enabled
        && matches!(prefs.auto_upgrade_policy.as_str(), "smart_split" | "auto_apply_zone1")
        && !system_patches.is_empty();

    let requires_user_feature_prompt = !optional_features.is_empty()
        && prefs.auto_upgrade_policy != "auto_apply_zone1";

    ClassifiedUpdate {
        target_ref: target_ref.to_string(),
        system_patches,
        system_auto_apply_silent,
        optional_features,
        requires_user_feature_prompt,
    }
}

pub fn load_prefs(msshi_home: &Path) -> ConnectPrefs {
    let path = msshi_home.join("connect-app-prefs.json");
    fs::read_to_string(&path)
        .ok()
        .and_then(|raw| serde_json::from_str::<ConnectPrefs>(&raw).ok())
        .map(ConnectPrefs::normalized)
        .unwrap_or_default()
}

pub fn save_prefs(msshi_home: &Path, prefs: &ConnectPrefs) -> Result<ConnectPrefs, String> {
    let clean = prefs.clone().normalized();
    let path = msshi_home.join("connect-app-prefs.json");
    let raw = serde_json::to_vec_pretty(&clean).map_err(|e| e.to_string())?;
    atomic_write_acl_safe(&path, &raw).map_err(|e| e.to_string())?;
    Ok(clean)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manual_mode_pins_chosen_version_and_auto_tracks_channel() {
        let manual = ConnectPrefs {
            auto_upgrade_enabled: false,
            selected_ref: "v3.1.2".to_string(),
            channel: "main".to_string(),
            ..Default::default()
        };
        assert_eq!(manual.resolve_target_ref("v3.2.0"), "v3.1.2");

        let auto_stable = ConnectPrefs {
            auto_upgrade_enabled: true,
            channel: "stable".to_string(),
            ..Default::default()
        };
        assert_eq!(auto_stable.resolve_target_ref("v3.2.0"), "v3.2.0");
    }

    #[test]
    fn smart_split_applies_system_patches_silently_and_prompts_for_features() {
        let prefs = ConnectPrefs::default(); // smart_split
        let changes = vec![
            "scripts/msshi_cli.py".to_string(),
            "hermes_plugins/msshi-auto-trim/plugin.py".to_string(),
            "skills/new-magic-skill/SKILL.md".to_string(),
            "optional/rea/SKILL.md".to_string(),
        ];
        let classified = classify_msshi_update("v3.2.0", &changes, &prefs);
        assert!(classified.system_auto_apply_silent);
        assert_eq!(classified.system_patches.len(), 2);
        assert!(classified.requires_user_feature_prompt);
        assert_eq!(classified.optional_features.len(), 2);
    }
}
