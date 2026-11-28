pub const APP_NAME: &str = "MSSHI Installer & Connect";
pub const APP_VERSION: &str = "1.0.2";
pub const UPSTREAM_OWNER: &str = "zidanaisar624";
pub const UPSTREAM_REPO: &str = "MSSHI-Mega-Semar-Sang-Hyang-Ismaya";
pub const UPSTREAM_FULL_NAME: &str = "zidanaisar624/MSSHI-Mega-Semar-Sang-Hyang-Ismaya";
pub const APP_REPO_URL: &str = "https://github.com/zidanaisar624/MSSHI-CONNECT";
pub const DOCTOR_REPO_URL: &str = "https://github.com/zidanaisar624/MSSHI-DOCTOR";
pub const HERMES_OFFICIAL_URL: &str = "https://hermes-agent.nousresearch.com/";
pub const SAWERIA_URL: &str = "https://saweria.co/muzas";
pub const DEFAULT_REF: &str = "v3.2.2";

pub const ALLOWED_GITHUB_HOSTS: &[&str] = &[
    "api.github.com",
    "raw.githubusercontent.com",
    "github.com",
    "codeload.github.com",
    "objects.githubusercontent.com",
    "release-assets.githubusercontent.com",
];

/// Zone 1: Core MSSHI directories allowed for managed install/upgrade after preview.
pub const ZONE1_DIRS: &[&str] = &[
    "scripts",
    "crates",
    "hermes_plugins",
    "configs",
    "skills",
    "optional",
    "templates",
    "docs",
    "portable",
    "assets",
    "adapters",
    "business-sage",
    ".claude-plugin",
    "notebooks",
];

/// Zone 2: User-owned sensitive configuration & persona files (NEVER auto-overwritten or read for secrets).
pub const ZONE2_FILENAMES: &[&str] = &[
    "soul.md",
    "config.yaml",
    "config.json",
    "profile.yaml",
    "ui-preferences.json",
    "user_creative_dna.json",
    ".env",
    "auth.json",
    "auth_keys.json",
    "state.db",
    "state.db-wal",
    "state.db-shm",
    "active_profile",
];

pub const ZONE2_EXTENSIONS: &[&str] = &[".pem", ".key", ".p12", ".pfx"];

/// Zone 3: Strict private local paths (SKIP TOTAL — never read, walked, or modified).
pub const ZONE3_PROTECTED_SEGMENTS: &[&str] = &[
    "1. private",
    "2. data diri atau perkembangan diri (np)",
    "msshi_recovery",
    "msshi-private",
];

pub const ZONE3_PROTECTED_FILENAMES: &[&str] = &[
    "nlm.exe",
    "notebooklm-mcp.exe",
    "rclone.conf",
];

pub const BOTS: &[&str] = &["yukino", "shizuka", "val", "kiki-chii", "shikaku"];
