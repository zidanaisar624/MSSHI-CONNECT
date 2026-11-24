use crate::constants::{
    ALLOWED_GITHUB_HOSTS, UPSTREAM_FULL_NAME, ZONE2_EXTENSIONS, ZONE2_FILENAMES,
    ZONE3_PROTECTED_FILENAMES, ZONE3_PROTECTED_SEGMENTS,
};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Write,
    path::{Component, Path, PathBuf},
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SafetyError {
    #[error("Path kosong atau tidak valid")]
    EmptyPath,
    #[error("Path UNC jaringan ditolak: {0}")]
    UncPathBlocked(String),
    #[error("Path traversal ('..') ditolak: {0}")]
    TraversalBlocked(String),
    #[error("Symlink/reparse point ditolak demi keamanan: {0}")]
    SymlinkBlocked(String),
    #[error("Akses ke Zona 2 (file user/kredensial) ditolak: {0}")]
    Zone2Protected(String),
    #[error("Akses ke Zona 3 (area privat) ditolak total: {0}")]
    Zone3Protected(String),
    #[error("URL tidak lolos allowlist GitHub HTTPS: {0}")]
    UrlBlocked(String),
    #[error("Kesalahan I/O: {0}")]
    Io(#[from] std::io::Error),
}

pub fn generate_csprng_token_hex() -> String {
    let mut bytes = [0u8; 32];
    if getrandom::getrandom(&mut bytes).is_err() {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let fallback = format!("msshi-{}-{nanos}", std::process::id());
        return sha256_hex(fallback.as_bytes());
    }
    hex::encode(bytes)
}

pub fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

pub fn validate_github_url(url: &str) -> Result<String, SafetyError> {
    let trimmed = url.trim();
    if !trimmed.starts_with("https://") {
        return Err(SafetyError::UrlBlocked(trimmed.to_string()));
    }
    let without_scheme = &trimmed["https://".len()..];
    let (host_port, rest) = match without_scheme.find('/') {
        Some(idx) => (&without_scheme[..idx], &without_scheme[idx..]),
        None => (without_scheme, "/"),
    };
    let host = host_port.split(':').next().unwrap_or("").to_ascii_lowercase();
    if !ALLOWED_GITHUB_HOSTS.contains(&host.as_str()) {
        return Err(SafetyError::UrlBlocked(trimmed.to_string()));
    }
    if host == "api.github.com" && !rest.starts_with(&format!("/repos/{UPSTREAM_FULL_NAME}")) {
        return Err(SafetyError::UrlBlocked(trimmed.to_string()));
    }
    if host == "raw.githubusercontent.com" && !rest.starts_with(&format!("/{UPSTREAM_FULL_NAME}/")) {
        return Err(SafetyError::UrlBlocked(trimmed.to_string()));
    }
    Ok(trimmed.to_string())
}

pub fn is_zone3_protected(path: &Path) -> bool {
    let normalized = path.to_string_lossy().replace('\\', "/").to_ascii_lowercase();
    for seg in ZONE3_PROTECTED_SEGMENTS {
        if normalized.split('/').any(|p| p == *seg) {
            return true;
        }
    }
    if normalized.contains("/data/bin/") || normalized.contains("/.config/rclone") || normalized.contains("/roaming/rclone") {
        return true;
    }
    if let Some(fname) = path.file_name().and_then(|s| s.to_str()) {
        let low = fname.to_ascii_lowercase();
        if ZONE3_PROTECTED_FILENAMES.contains(&low.as_str()) {
            return true;
        }
    }
    false
}

pub fn is_zone2_protected(path: &Path) -> bool {
    if let Some(fname) = path.file_name().and_then(|s| s.to_str()) {
        let low = fname.to_ascii_lowercase();
        if ZONE2_FILENAMES.contains(&low.as_str()) {
            return true;
        }
        if ZONE2_EXTENSIONS.iter().any(|ext| low.ends_with(ext)) {
            return true;
        }
    }
    false
}

pub fn validate_zone1_relative_path(rel: &str) -> Result<PathBuf, SafetyError> {
    let cleaned = rel.trim().replace('\\', "/");
    if cleaned.is_empty() {
        return Err(SafetyError::EmptyPath);
    }
    if cleaned.starts_with("//") || cleaned.starts_with('/') || cleaned.contains(':') {
        return Err(SafetyError::UncPathBlocked(rel.to_string()));
    }
    let path = PathBuf::from(&cleaned);
    for comp in path.components() {
        match comp {
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(SafetyError::TraversalBlocked(rel.to_string()));
            }
            _ => {}
        }
    }
    if is_zone3_protected(&path) {
        return Err(SafetyError::Zone3Protected(rel.to_string()));
    }
    if is_zone2_protected(&path) {
        return Err(SafetyError::Zone2Protected(rel.to_string()));
    }
    Ok(path)
}

pub fn reject_symlinks(path: &Path) -> Result<(), SafetyError> {
    if let Ok(meta) = fs::symlink_metadata(path) {
        if meta.file_type().is_symlink() {
            return Err(SafetyError::SymlinkBlocked(path.display().to_string()));
        }
    }
    Ok(())
}

/// Atomic ACL-preserving file writer: writes to a sibling temporary file in the target directory
/// so Windows inherited user NTFS ACLs are never stripped (avoids NamedTemporaryFile Admin-Lock bug).
pub fn atomic_write_acl_safe(target: &Path, content: &[u8]) -> Result<(), SafetyError> {
    if target.to_string_lossy().starts_with(r"\\") {
        return Err(SafetyError::UncPathBlocked(target.display().to_string()));
    }
    if is_zone3_protected(target) {
        return Err(SafetyError::Zone3Protected(target.display().to_string()));
    }
    if is_zone2_protected(target) {
        return Err(SafetyError::Zone2Protected(target.display().to_string()));
    }
    reject_symlinks(target)?;
    let parent = target.parent().ok_or(SafetyError::EmptyPath)?;
    fs::create_dir_all(parent)?;
    reject_symlinks(parent)?;

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let tmp_name = format!(".msshi-tmp-{}-{nonce}", std::process::id());
    let tmp_path = parent.join(tmp_name);

    {
        let mut f = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp_path)?;
        f.write_all(content)?;
        f.sync_all()?;
    }

    if target.exists() {
        let _ = fs::remove_file(target);
    }
    if let Err(err) = fs::rename(&tmp_path, target) {
        let _ = fs::remove_file(&tmp_path);
        return Err(SafetyError::Io(err));
    }
    Ok(())
}

/// Detects whether Windows process is running as Elevated Administrator (Anti-Admin-Lock guard).
pub fn is_windows_elevated_admin() -> bool {
    if !cfg!(windows) {
        return false;
    }
    Command::new("fltmc")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Computes safe Auto-Temp Redirect path on target drive (e.g. D:\HermesData\temp) when Drive C: is tight.
pub fn resolve_auto_temp_dir(target_msshi_home: &Path, drive_c_free_gb: Option<f64>) -> PathBuf {
    let target_str = target_msshi_home.to_string_lossy();
    let drive_prefix = if target_str.len() >= 2 && target_str.as_bytes()[1] == b':' {
        target_str[..2].to_ascii_uppercase()
    } else {
        "C:".to_string()
    };
    if drive_prefix != "C:" && drive_c_free_gb.unwrap_or(0.0) < 5.0 {
        return PathBuf::from(format!(r"{drive_prefix}\HermesData\temp"));
    }
    if drive_prefix != "C:" {
        return PathBuf::from(format!(r"{drive_prefix}\HermesData\temp"));
    }
    std::env::temp_dir().join("msshi-connect-temp")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zone2_and_zone3_and_traversal_are_blocked() {
        assert!(validate_zone1_relative_path("../evil.txt").is_err());
        assert!(validate_zone1_relative_path(r"\\server\share\a").is_err());
        assert!(validate_zone1_relative_path("SOUL.md").is_err());
        assert!(validate_zone1_relative_path("profiles/yukino/config.yaml").is_err());
        assert!(validate_zone1_relative_path("profiles/yukino/.env").is_err());
        assert!(validate_zone1_relative_path("1. Private/secret.md").is_err());
        assert!(validate_zone1_relative_path("2. Data Diri atau Perkembangan Diri (NP)/a.png").is_err());
        assert!(validate_zone1_relative_path("scripts/msshi_cli.py").is_ok());
    }

    #[test]
    fn allowlist_blocks_non_official_urls() {
        assert!(validate_github_url(
            "https://api.github.com/repos/zidanaisar624/MSSHI-Mega-Semar-Sang-Hyang-Ismaya/releases"
        ).is_ok());
        assert!(validate_github_url("http://api.github.com/repos/zidanaisar624/MSSHI-Mega-Semar-Sang-Hyang-Ismaya/releases").is_err());
        assert!(validate_github_url("https://api.github.com/repos/other/repo/releases").is_err());
        assert!(validate_github_url("https://evil.example.com/payload").is_err());
    }

    #[test]
    fn auto_temp_redirects_to_target_drive_when_c_is_tight() {
        let p = resolve_auto_temp_dir(Path::new(r"D:\MSSHI"), Some(4.2));
        assert_eq!(p, PathBuf::from(r"D:\HermesData\temp"));
    }

    #[test]
    fn atomic_writer_creates_and_updates_zone1_file() {
        let dir = std::env::temp_dir().join(format!("msshi-atomic-test-{}", std::process::id()));
        let target = dir.join("VERSION.json");
        atomic_write_acl_safe(&target, b"{\"version\":\"3.1.2\"}").unwrap();
        assert_eq!(fs::read_to_string(&target).unwrap(), "{\"version\":\"3.1.2\"}");
        let _ = fs::remove_dir_all(&dir);
    }
}
