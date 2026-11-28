use crate::safety::{is_windows_elevated_admin, resolve_auto_temp_dir};
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriveInfo {
    pub letter: String,
    pub free_gb: f64,
    pub total_gb: f64,
    pub low_space_warning: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemProbe {
    pub os: String,
    pub total_ram_gb: f64,
    pub available_ram_gb: f64,
    pub recommend_light_mode: bool,
    pub elevated_admin: bool,
    pub drives: Vec<DriveInfo>,
    pub hermes_detected: bool,
    pub hermes_home: String,
    pub msshi_home: String,
    pub installed_msshi_version: Option<String>,
    pub doctor_detected: bool,
    pub doctor_path: Option<String>,
    pub recommended_temp_dir: String,
    pub ml_preflight_ok: bool,
    pub ml_preflight_note: String,
}

fn detect_drives_windows() -> Vec<DriveInfo> {
    if !cfg!(windows) {
        return vec![];
    }
    let script = "Get-PSDrive -PSProvider FileSystem | Where-Object { $_.Name -match '^[C-Z]$' -and $_.Free -ne $null } | ForEach-Object { '{0}|{1}|{2}' -f $_.Name, [math]::Round($_.Free/1GB,2), [math]::Round(($_.Free+$_.Used)/1GB,2) }";
    let out = Command::new("powershell")
        .args(["-NoProfile", "-Command", script])
        .output();
    let Ok(output) = out else {
        return vec![];
    };
    let text = String::from_utf8_lossy(&output.stdout);
    let mut drives = Vec::new();
    for line in text.lines() {
        let parts: Vec<&str> = line.trim().split('|').collect();
        if parts.len() == 3 {
            let free = parts[1].parse::<f64>().unwrap_or(0.0);
            let total = parts[2].parse::<f64>().unwrap_or(0.0);
            drives.push(DriveInfo {
                letter: format!("{}:", parts[0]),
                free_gb: free,
                total_gb: total,
                low_space_warning: free < 5.0,
            });
        }
    }
    drives
}

fn detect_ram_gb() -> (f64, f64) {
    if !cfg!(windows) {
        return (8.0, 4.0);
    }
    let script = "$os = Get-CimInstance Win32_OperatingSystem; '{0}|{1}' -f ([math]::Round($os.TotalVisibleMemorySize/1MB,2)), ([math]::Round($os.FreePhysicalMemory/1MB,2))";
    let out = Command::new("powershell")
        .args(["-NoProfile", "-Command", script])
        .output();
    let Ok(output) = out else {
        return (8.0, 4.0);
    };
    let text = String::from_utf8_lossy(&output.stdout);
    let parts: Vec<&str> = text.trim().split('|').collect();
    if parts.len() == 2 {
        return (
            parts[0].parse().unwrap_or(8.0),
            parts[1].parse().unwrap_or(4.0),
        );
    }
    (8.0, 4.0)
}

fn read_installed_msshi_version(msshi_home: &Path) -> Option<String> {
    let candidates = [
        msshi_home.join("current").join("VERSION.json"),
        msshi_home.join("VERSION.json"),
    ];
    for c in candidates {
        if let Ok(raw) = std::fs::read_to_string(&c) {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&raw) {
                if let Some(v) = val.get("version").and_then(|s| s.as_str()) {
                    return Some(v.to_string());
                }
            }
        }
    }
    None
}

pub fn detect_doctor_path() -> Option<PathBuf> {
    let candidates = [
        PathBuf::from(r"D:\MSSHI DOCTOR\MSSHI-Doctor-Portable-win-x64.exe"),
        PathBuf::from(r"D:\MSSHI DOCTOR\MSSHI-Doctor.exe"),
        PathBuf::from(r"D:\MSSHI DOCTOR\MSSHI_Doctor.bat"),
        PathBuf::from(r"C:\MSSHI DOCTOR\MSSHI-Doctor-Portable-win-x64.exe"),
        PathBuf::from(r"C:\MSSHI DOCTOR\MSSHI-Doctor.exe"),
    ];
    candidates.into_iter().find(|p| p.is_file())
}

/// Fast zero-subprocess resolver for default MSSHI_HOME so lightweight HTTP endpoints
/// (/api/health, /api/catalog, /api/prefs, /api/receipts) never spawn PowerShell.
pub fn resolve_default_msshi_home() -> PathBuf {
    if let Some(home) = std::env::var_os("MSSHI_HOME").filter(|p| !p.is_empty()) { return PathBuf::from(home); }
    if cfg!(windows) && Path::new(r"D:\").is_dir() {
        return PathBuf::from(r"D:\MSSHI");
    }
    std::env::var_os("USERPROFILE")
        .map(|u| PathBuf::from(u).join("MSSHI"))
        .unwrap_or_else(|| PathBuf::from(r"C:\MSSHI"))
}

pub fn collect_system_probe(explicit_msshi: Option<&Path>, explicit_hermes: Option<&Path>) -> SystemProbe {
    let drives = detect_drives_windows();
    let has_d = drives.iter().any(|d| d.letter == "D:");
    let c_free = drives.iter().find(|d| d.letter == "C:").map(|d| d.free_gb);
    let d_free = drives.iter().find(|d| d.letter == "D:").map(|d| d.free_gb);

    let msshi_home = explicit_msshi
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("MSSHI_HOME").map(PathBuf::from))
        .unwrap_or_else(|| {
            if has_d {
                PathBuf::from(r"D:\MSSHI")
            } else {
                std::env::var_os("USERPROFILE")
                    .map(|u| PathBuf::from(u).join("MSSHI"))
                    .unwrap_or_else(|| PathBuf::from(r"C:\MSSHI"))
            }
        });

    let hermes_home = explicit_hermes
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HERMES_HOME").map(PathBuf::from))
        .unwrap_or_else(|| {
            if has_d {
                PathBuf::from(r"D:\HermesData")
            } else {
                std::env::var_os("USERPROFILE")
                    .map(|u| PathBuf::from(u).join("HermesData"))
                    .unwrap_or_else(|| PathBuf::from(r"C:\HermesData"))
            }
        });

    let (total_ram_gb, available_ram_gb) = detect_ram_gb();
    let hermes_detected = hermes_home.join("hermes-agent").is_dir()
        || hermes_home.join("profiles").is_dir()
        || hermes_home.join("bin").join("hermes.exe").is_file();
    let doctor_path = detect_doctor_path();
    let temp_dir = resolve_auto_temp_dir(&msshi_home, c_free);
    let target_free = if has_d { d_free.unwrap_or(0.0) } else { c_free.unwrap_or(0.0) };
    let ml_preflight_ok = target_free >= 3.0 && available_ram_gb >= 2.0;
    let ml_preflight_note = if ml_preflight_ok {
        format!("Siap untuk add-on opsional (bebas {:.1} GB, RAM tersedia {:.1} GB).", target_free, available_ram_gb)
    } else {
        format!(
            "Ruang disk ({:.1} GB) atau RAM tersedia ({:.1} GB) sempit untuk TimesFM/OmniVoice; disarankan Mode Hemat 4 GB.",
            target_free, available_ram_gb
        )
    };

    SystemProbe {
        os: format!("{} ({})", std::env::consts::OS, std::env::consts::ARCH),
        total_ram_gb,
        available_ram_gb,
        recommend_light_mode: total_ram_gb <= 6.0,
        elevated_admin: is_windows_elevated_admin(),
        drives,
        hermes_detected,
        hermes_home: hermes_home.display().to_string(),
        msshi_home: msshi_home.display().to_string(),
        installed_msshi_version: read_installed_msshi_version(&msshi_home),
        doctor_detected: doctor_path.is_some(),
        doctor_path: doctor_path.map(|p| p.display().to_string()),
        recommended_temp_dir: temp_dir.display().to_string(),
        ml_preflight_ok,
        ml_preflight_note,
    }
}
