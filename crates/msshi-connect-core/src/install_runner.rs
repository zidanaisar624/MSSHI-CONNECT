use crate::safety::{atomic_write_acl_safe, sha256_hex};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallPlanRequest {
    pub target_ref: String,
    pub commit_sha: Option<String>,
    pub msshi_home: String,
    pub hermes_home: String,
    pub mode: String,
    pub platform: String,
    pub default_bot: String,
    pub light_low_ram: bool,
    pub copy_pdf_manual: bool,
    pub pdf_version: String,
    pub addons: Vec<String>,
    pub context_sources: Vec<String>,
    pub connect_mode: String, // "skip" | "auto-reconnect" | "select"
    pub local_source_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallDryRun {
    pub target_ref: String,
    pub commit_sha: Option<String>,
    pub msshi_home: String,
    pub hermes_home: String,
    pub temp_redirect: String,
    pub steps: Vec<String>,
    pub command_args: Vec<String>,
    pub zone2_guarantee: String,
    pub estimated_download_mb: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectReceipt {
    pub receipt_id: String,
    pub created_epoch: u64,
    pub action: String,
    pub target_ref: String,
    pub commit_sha: Option<String>,
    pub msshi_home: String,
    pub hermes_home: String,
    pub mode: String,
    pub state: String,
    pub sha256: String,
    pub can_rollback: bool,
}

pub fn build_dry_run(req: &InstallPlanRequest) -> InstallDryRun {
    let mut args = vec![
        "setup".to_string(),
        "--mode".to_string(),
        req.mode.clone(),
        "--platform".to_string(),
        req.platform.clone(),
        "--default-bot".to_string(),
        req.default_bot.clone(),
        "--yes".to_string(),
        "--stage-auto-trim".to_string(),
        "--connect-apps".to_string(),
        req.connect_mode.clone(),
    ];
    if req.light_low_ram {
        args.push("--light".to_string());
        args.push("--low-ram".to_string());
    }
    if req.addons.iter().any(|a| a == "rea") {
        args.push("--with-rea".to_string());
    }
    if req.addons.iter().any(|a| a == "imagination-studio") {
        args.push("--with-imagination-studio".to_string());
    }
    if req.addons.iter().any(|a| a == "mcp-for-blender") {
        args.push("--with-blender-mcp".to_string());
    }
    if req.addons.iter().any(|a| a == "raea") {
        args.push("--with-raea".to_string());
    }
    if req.addons.iter().any(|a| a == "lemo-opuscar") {
        args.push("--with-lemo-opuscar".to_string());
    }
    if req.addons.iter().any(|a| a == "meshy-3d-generation") {
        args.push("--with-meshy-3d".to_string());
    }
    if !req.copy_pdf_manual {
        args.push("--no-pdf".to_string());
    }

    let mut est_mb: u32 = if req.light_low_ram { 35 } else { 48 };
    for addon in &req.addons {
        match addon.as_str() {
            "rea" => est_mb += 180,
            "timesfm" => est_mb += 1800,
            "omnivoice" => est_mb += 2000,
            "learnloop" | "dreaming" | "mcp-for-blender" | "lemo-opuscar" => est_mb += 2,
            "raea" | "imagination-studio" | "meshy-3d-generation" => est_mb += 1,
            _ => {}
        }
    }

    let drive = req.msshi_home.get(..2).unwrap_or("D:").to_ascii_uppercase();
    let temp_redirect = if drive != "C:" {
        format!(r"{drive}\HermesData\temp")
    } else {
        "%TEMP%\\msshi-connect-temp".to_string()
    };

    let mut steps = vec![
        format!("Arahkan TEMP/TMP ke {temp_redirect} agar Drive C: aman."),
        format!(
            "Sinkronkan Zona 1 MSSHI ({}) ke {} (backup receipt otomatis).",
            req.target_ref, req.msshi_home
        ),
        format!(
            "Jalankan msshi {} (Hermes Home: {}).",
            args.join(" "),
            req.hermes_home
        ),
    ];
    if !req.context_sources.is_empty() {
        steps.push(format!(
            "Fetch on-demand context-sources: {}",
            req.context_sources.join(", ")
        ));
    }
    if req.copy_pdf_manual {
        steps.push(format!("Salin Buku Panduan PDF Klien (kanal: {}).", req.pdf_version));
    }

    InstallDryRun {
        target_ref: req.target_ref.clone(),
        commit_sha: req.commit_sha.clone(),
        msshi_home: req.msshi_home.clone(),
        hermes_home: req.hermes_home.clone(),
        temp_redirect,
        steps,
        command_args: args,
        zone2_guarantee: "DIKUNCI: Provider, model, endpoint, API key, .env, auth.json, SOUL.md kustom, dan Zona 3 tidak pernah diubah.".to_string(),
        estimated_download_mb: est_mb,
    }
}

pub fn receipts_dir(msshi_home: &Path) -> PathBuf {
    msshi_home.join("connect-receipts")
}

pub fn write_receipt(msshi_home: &Path, req: &InstallPlanRequest, state: &str) -> Result<ConnectReceipt, String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let epoch = now.as_secs();
    let sub_ms = now.subsec_millis();
    let base_id = format!("connect-{epoch}");
    let dir = receipts_dir(msshi_home);
    let receipt_id = if dir.join(format!("{base_id}.json")).exists() {
        format!("connect-{epoch}-{sub_ms:03}")
    } else {
        base_id
    };
    let seed = format!("{receipt_id}:{}:{}:{}", req.target_ref, req.mode, req.msshi_home);
    let receipt = ConnectReceipt {
        receipt_id: receipt_id.clone(),
        created_epoch: epoch,
        action: "msshi-connect-install".to_string(),
        target_ref: req.target_ref.clone(),
        commit_sha: req.commit_sha.clone(),
        msshi_home: req.msshi_home.clone(),
        hermes_home: req.hermes_home.clone(),
        mode: req.mode.clone(),
        state: state.to_string(),
        sha256: sha256_hex(seed.as_bytes()),
        can_rollback: state == "APPLIED",
    };
    let path = receipts_dir(msshi_home).join(format!("{receipt_id}.json"));
    let raw = serde_json::to_vec_pretty(&receipt).map_err(|e| e.to_string())?;
    atomic_write_acl_safe(&path, &raw).map_err(|e| e.to_string())?;
    Ok(receipt)
}

pub fn list_receipts(msshi_home: &Path) -> Vec<ConnectReceipt> {
    let dir = receipts_dir(msshi_home);
    let Ok(rd) = fs::read_dir(&dir) else {
        return vec![];
    };
    let mut items: Vec<ConnectReceipt> = rd
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("json"))
        .filter_map(|e| fs::read_to_string(e.path()).ok())
        .filter_map(|raw| serde_json::from_str::<ConnectReceipt>(&raw).ok())
        .collect();
    items.sort_by(|a, b| b.created_epoch.cmp(&a.created_epoch));
    items
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dry_run_builds_expected_flags_for_light_mode_and_addons() {
        let req = InstallPlanRequest {
            target_ref: "v3.1.2".into(),
            commit_sha: Some("fea245c".into()),
            msshi_home: r"D:\MSSHI".into(),
            hermes_home: r"D:\HermesData".into(),
            mode: "muza-preset".into(),
            platform: "hermes".into(),
            default_bot: "auto".into(),
            light_low_ram: true,
            copy_pdf_manual: true,
            pdf_version: "v3.1.2".into(),
            addons: vec!["rea".into(), "imagination-studio".into()],
            context_sources: vec!["rust-anything".into()],
            connect_mode: "auto-reconnect".into(),
            local_source_path: None,
        };
        let dry = build_dry_run(&req);
        assert!(dry.command_args.contains(&"--light".to_string()));
        assert!(dry.command_args.contains(&"--with-rea".to_string()));
        assert!(dry.command_args.contains(&"--with-imagination-studio".to_string()));
        assert_eq!(dry.temp_redirect, r"D:\HermesData\temp");
    }
}
